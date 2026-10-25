#!/usr/bin/env python3
"""Inputs for the 3-class task: final messages only, with the sent text of hcom-send endings.

Usage: python3 -I final_inputs.py [--hcom BIN] OUT EXPORT...
  EXPORT  private export stems from build_eval_set.py (candidates, candidates-<batch>, ...)
  OUT     new private stem; written once (identical rebuild = no-op)

A message is final when hcom recorded the instance going active -> listening between this
exchange and the next one (`final: true`); a next input without that event interrupted a
running turn (`final: false`); an instance hcom did not know then is `final: "unknown"`.
Input text: final_text when present; a tool-free turn's whole reply; for a turn that ended on
a tool call, the reply text plus the last command when it is an `hcom send` (the agent's actual
final message), else the reply text alone. Last 3000 characters, as the contract.
An exchange is used when its timestamp and text reproduce the export id, else when exactly one
transcript's exchange still contains the exported text (counted as ambiguous otherwise); a bare
instance name is also looked up in its earlier transcripts (a later session shifts the name).
Counts go to results/final-inputs-OUT-summary.json.
"""
import argparse
import bisect
import collections
import datetime
import hashlib
import json
import os
import pathlib
import re
import subprocess
import sys

HERE = pathlib.Path(__file__).resolve().parent
RESULTS = HERE / "results"  # metrics only, committed
MAX_CHARS = 3000
CS_CHARS = re.compile(r"[ěščřžýáíéůúťďň]")
# `hcom send` run as a command, optionally via `env [-C DIR]`, a path to the binary or `--name NAME`.
HCOM_SEND = re.compile(r"\s*(?:env\s+(?:-C\s+\S+\s+)?)?((?:\S*/)?hcom(?:\s+--name\s+\S+)?\s+send\s)")


HEREDOC = re.compile(r"<<(?!<)(-?)\s*(['\"]?)(\w+)\2")


def command_starts(command):
    """Offsets where a shell command can start: 0 and after each separator outside quotes,
    comments and heredoc bodies."""
    starts, quote, escaped, heredocs, i = [0], None, False, [], 0
    while i < len(command):
        ch = command[i]
        if escaped:
            escaped = False
        elif ch == "\\" and quote != "'":
            escaped = True
        elif quote:
            quote = None if ch == quote else quote
        elif ch in "'\"":
            quote = ch
        elif ch == "#" and (i == 0 or command[i - 1].isspace() or command[i - 1] in ";&|()"):
            i = command.find("\n", i)  # the comment ends at the newline, itself a separator
            if i < 0:
                break
            continue
        elif ch == "<" and command[i - 1:i] != "<" and (m := HEREDOC.match(command, i)):
            heredocs.append((m.group(3), r"\t*" if m.group(1) else ""))
            i = m.end()
            continue
        elif ch == "\n" and heredocs:
            # Skip each pending heredoc body up to its terminator line.
            for word, tabs in heredocs:  # only `<<-` allows leading tabs on the terminator
                end = re.compile(rf"^{tabs}{re.escape(word)}$", re.M).search(command, i + 1)
                i = end.end() if end else len(command)
            heredocs = []
            starts.append(i + 1)
        elif ch in ";&|(\n":
            starts.append(i + 1)
        i += 1
    return starts
# Shell commands inside a Codex code-mode `exec` script: tools.exec_command({"cmd": "..."}).
EXEC_CMD = re.compile(r'"cmd"\s*:\s*("(?:[^"\\]|\\.)*")')


def hcom_send(tool):
    """The tool's command when it is an `hcom send`; for an `exec` script, its last shell command."""
    command = tool.get("command") or ""
    if tool.get("name") == "exec":
        cmds = EXEC_CMD.findall(command)
        command = json.loads(cmds[-1]) if cmds else ""
    elif tool.get("name") != "Bash":
        return None
    for start in command_starts(command):
        if match := HCOM_SEND.match(command, start):
            return command[match.start(1):]
    return None


def final_input(ex):
    """(input_kind, text) of a `hcom transcript --json --detailed` exchange, as the docstring says."""
    last = (ex.get("tools") or [{}])[-1]
    if ex.get("final_text"):
        kind, text = "final_text", ex["final_text"]
    elif not ex.get("tools"):
        kind, text = "tool_free", ex.get("action") or ""
    elif send := hcom_send(last):
        kind, text = "hcom_send", f"{ex.get('action') or ''}\n\n{send}"
    else:
        kind, text = "tool_ended_other", ex.get("action") or ""
    return kind, text.strip()[-MAX_CHARS:]


class PrivateDir:
    """$HCOM_DIR/tmp/idle-classifier: message texts live only here, never in the repository."""

    def __init__(self):
        root = pathlib.Path(os.environ.get("HCOM_DIR", pathlib.Path.home() / ".hcom")).expanduser()
        self.path = (root / "tmp" / "idle-classifier").resolve()
        if self.path.is_relative_to(HERE.parents[1]):
            sys.exit(f"refusing private data directory inside the repository: {self.path}")

    def __truediv__(self, name):
        if pathlib.Path(name).name != name:
            sys.exit(f"unsafe file name: {name!r}")
        path = self.path / name
        if path.is_symlink():
            sys.exit(f"refusing symlinked private file: {path}")
        return path


OUT = PrivateDir()


def read_jsonl(path):
    with open(path, encoding="utf-8") as f:
        return [json.loads(line) for line in f if line.strip()]


def ts(value):
    try:
        parsed = datetime.datetime.fromisoformat(value.replace("Z", "+00:00"))
    except (AttributeError, ValueError):
        return None
    return parsed if parsed.tzinfo else parsed.replace(tzinfo=datetime.timezone.utc)


def events(hcom, *args):
    out = subprocess.run([hcom, "events", "--all", "--full", "--last", "10000000", *args],
                         capture_output=True, text=True, timeout=600)
    if out.returncode != 0:
        sys.exit(f"hcom events failed (exit {out.returncode})")
    return [json.loads(line) for line in out.stdout.splitlines() if line.startswith("{")]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--hcom", default="hcom")
    ap.add_argument("out")
    ap.add_argument("exports", nargs="+")
    args = ap.parse_args()
    if not all(re.fullmatch(r"[\w.-]+", a) for a in [args.out, *args.exports]):
        sys.exit("OUT and EXPORT must be plain private file stems")

    idle = collections.defaultdict(list)  # instance -> sorted active->listening times
    for ev in events(args.hcom, "--type", "status", "--status", "listening",
                     "--sql", "json_extract(data,'$.old_status')='active'"):
        idle[ev["instance"]].append(ts(ev["ts"]))
    for times in idle.values():
        times.sort()
    known_since, paths = {}, {}
    inst_paths = collections.defaultdict(list)  # instance -> its transcript paths, newest first
    for ev in events(args.hcom, "--type", "life"):
        when = ts(ev["ts"])
        if ev["instance"] not in known_since or when < known_since[ev["instance"]]:
            known_since[ev["instance"]] = when
        path = ((ev.get("data") or {}).get("snapshot") or {}).get("transcript_path")
        if path:
            paths[hashlib.sha1(path.encode()).hexdigest()[:8]] = path
            if path not in inst_paths[ev["instance"]]:
                inst_paths[ev["instance"]].insert(0, path)

    rows, kinds, cache = [], collections.Counter(), {}

    def exchanges(target):
        key = tuple(target)
        if key not in cache:
            out = subprocess.run([args.hcom, "transcript", *target, "1-", "--json", "--detailed"],
                                 capture_output=True, text=True, timeout=120)
            cache[key] = {e["position"]: e for e in json.loads(out.stdout)} if out.returncode == 0 else {}
        return cache[key]

    for export in args.exports:
        for c in read_jsonl(OUT / f"{export}.jsonl"):
            source, pos = c["source_ref"].rsplit("#", 1)
            inst, _, phash = source.partition("@")
            if phash:
                targets = [["--path", paths[phash]]] if phash in paths else []
            else:
                # A bare name reached the newest incarnation at export time; a later session
                # of the same name shifts it, so earlier transcripts of the instance are tried too.
                targets = [[inst]] + [["--path", p] for p in inst_paths.get(inst, [])]
            # The export id hashes ref, exchange timestamp and full text (final_text, or the whole
            # action before a parser gave the turn a final_text): a hash match is that exchange.
            # Otherwise the exchange must still contain the exported text, in exactly one transcript.
            hashed, contained = [], []
            for target in targets:
                found = exchanges(target)
                cand = found.get(int(pos))
                if not cand or int(pos) + 1 not in found:
                    continue
                pair = (cand, found[int(pos) + 1])
                texts = {(cand.get(f) or "").strip() for f in ("final_text", "action")}
                if any(hashlib.sha1(f"{c['source_ref']}\n{cand.get('timestamp')}\n{t}".encode())
                       .hexdigest()[:12] == c["id"] for t in texts):
                    hashed.append(pair)
                elif c["text"] in (cand.get("action") or ""):
                    contained.append(pair)
            # The bare name and a --path can reach the same file, and a fork repeats its parent's
            # history: identical exchanges with the same next exchange are one match.
            matches = list({(a.get("timestamp"), a.get("action"), b.get("timestamp")): (a, b)
                            for a, b in hashed or contained}.values())
            if len(matches) != 1:
                kinds["ambiguous" if matches else "missing"] += 1
                continue
            kinds["id_verified" if hashed else "text_matched"] += 1
            ex, nxt = matches[0]
            start, end = ts(ex.get("timestamp")), ts(nxt.get("timestamp"))
            if not start or not end or inst not in known_since or known_since[inst] > start:
                final = "unknown"
            else:
                times = idle.get(inst, [])
                i = bisect.bisect_right(times, start)
                final = i < len(times) and times[i] < end
            kind, text = final_input(ex)
            kinds[f"{kind}/final={final}"] += 1
            rows.append({"id": c["id"], "text": text, "lang": "cs" if CS_CHARS.search(text.lower()) else "en",
                         "final": final, "input_kind": kind, "cli": c["cli"], "export": export,
                         "source_ref": c["source_ref"]})

    content = "".join(json.dumps(r, ensure_ascii=False) + "\n" for r in rows)
    path = OUT / f"{args.out}.jsonl"
    if path.exists():
        if path.read_text(encoding="utf-8") != content:
            sys.exit(f"{path.name} exists with different content; use a new OUT name")
    else:
        path.write_text(content, encoding="utf-8")
    summary = {"out": args.out, "exports": args.exports, "rows": len(rows),
               "final": dict(collections.Counter(str(r["final"]) for r in rows)),
               "by_kind": dict(sorted(kinds.items()))}
    (RESULTS / f"final-inputs-{args.out}-summary.json").write_text(json.dumps(summary, indent=2) + "\n")
    print(json.dumps(summary, indent=2))


if __name__ == "__main__":
    main()

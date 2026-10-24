#!/usr/bin/env python3
"""Build the weak-labelled real-data eval set for the idle-agent classifier.

Usage: python3 -I build_eval_set.py [--hcom BIN] [--batch NAME]   (BIN needs transcript final_text)

Every exchange that has a following exchange is one idle point: its final assistant text is the
classifier input, the next user turn gives the weak label. Weak labels only pre-sort; `label`
stays null until a gold labelling pass. Message texts stay in $HCOM_DIR/tmp/idle-classifier/.
Without --batch (first export, refused when it exists): candidates.jsonl (all) and cases.jsonl
(balanced sample), counts in results/dataset-summary.json.
With --batch NAME: only idle points whose text is in no earlier export (candidates.jsonl,
candidates-*.jsonl) go to candidates-NAME.jsonl, counts in results/dataset-NAME-summary.json.
With --batch NAME --older N: the sources are all stopped incarnations' transcript files read
via `hcom transcript --path` (shared session databases skipped); a random sample (seed 42) of
N new idle points is kept.
"""
import argparse
import collections
import datetime
import hashlib
import json
import os
import pathlib
import random
import re
import subprocess
import sys

HERE = pathlib.Path(__file__).resolve().parent
RESULTS = HERE / "results"  # metrics only, committed
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
HCOM = "hcom"
MAX_CHARS = 3000  # contract input: last 3000 characters
PER_LABEL = 60
LABELS = ("done", "needs_user", "blocked", "waiting_agent")

CS_CHARS = re.compile(r"[ěščřžýáíéůúťďň]")
BLOCKED = re.compile(
    r"permission denied|access denied|not permitted|forbidden|unreachable|connection refused"
    r"|no space left|cannot (access|reach|connect|continue|proceed)|can't (access|reach|continue)"
    r"|blocked on|i'?m blocked|missing (credential|token|access|permission)"
    r"|nemám (přístup|oprávnění|práva)|bez (přístupu|oprávnění)|nelze pokračovat|nemohu pokračovat"
    r"|nedostupn|zablokovan|blokuje mě|chybí (přístup|oprávnění|token|heslo)",
    re.I,
)
WAITING = re.compile(
    r"waiting for (a |the )?(reply|response|answer|@)|wait(ing)? for @\w+|until @\w+ (replies|responds)"
    r"|once @\w+ (replies|responds)|čekám na|počkám na|až @\w+ odpoví|až odpoví",
    re.I,
)


def hcom_json_lines(*args):
    out = subprocess.run([HCOM, *args], capture_output=True, text=True, timeout=120)
    return [json.loads(line) for line in out.stdout.splitlines() if line.startswith("{")]


def agents():
    """name -> tool for stopped and live instances, newest incarnation wins (as transcript does)."""
    found = {}
    stops = hcom_json_lines("events", "--type", "life", "--action", "stopped", "--last", "100000", "--full")
    for ev in sorted(stops, key=lambda e: e["id"]):
        snap = ev.get("data", {}).get("snapshot") or {}
        if snap.get("parent_name"):
            continue  # subagent rows
        found[ev["instance"]] = snap.get("tool") or ""
    live = subprocess.run([HCOM, "list", "--json"], capture_output=True, text=True, timeout=60)
    for row in json.loads(live.stdout or "[]"):
        if not row.get("parent_name"):
            found[row["name"]] = row.get("tool") or ""
    return {n: t for n, t in found.items() if t and t != "adhoc"}


SHARED_DB_TOOLS = {"opencode", "kilo", "hermes"}  # one database for all sessions: no --path


def older_transcripts():
    """(instance, tool, path) of every stopped incarnation whose transcript file still exists."""
    found = {}
    stops = hcom_json_lines("events", "--type", "life", "--action", "stopped", "--last", "100000", "--full")
    for ev in stops:
        snap = ev.get("data", {}).get("snapshot") or {}
        path, tool = snap.get("transcript_path") or "", snap.get("tool") or ""
        if snap.get("parent_name") or not path or tool in ("", "adhoc", *SHARED_DB_TOOLS):
            continue
        if os.path.isfile(path):
            found.setdefault(path, (ev["instance"], tool))
    return [(inst, tool, path) for path, (inst, tool) in sorted(found.items())]


def next_kind(user):
    text = user.lstrip()
    if text.startswith("<hcom>"):
        return "hcom"
    if not text or text.startswith("<"):
        return "system"  # task notifications, reminders, slash-command wrappers
    return "human"


def parse_ts(value):
    try:
        return datetime.datetime.fromisoformat(value.replace("Z", "+00:00")).timestamp()
    except (AttributeError, ValueError):
        return None


def weak_label(text, kind):
    tail = text[-600:]
    question = "?" in text[-400:]
    if BLOCKED.search(tail):
        return "blocked"  # rule: an obstacle is blocked even when it ends with a question
    if kind == "human" and question:
        return "needs_user"
    if WAITING.search(tail):
        return "waiting_agent"
    if question:
        return "needs_user"
    return "done"


def read_jsonl(path):
    with open(path, encoding="utf-8") as f:
        return [json.loads(line) for line in f if line.strip()]


def main():
    global HCOM
    ap = argparse.ArgumentParser()
    ap.add_argument("--hcom", default="hcom")
    ap.add_argument("--batch", help="name of an incremental export, e.g. 2026-10-24b")
    ap.add_argument("--older", type=int, metavar="N",
                    help="with --batch: read every stopped incarnation's transcript file "
                         "(hcom transcript --path) and keep a random sample of N new idle points")
    args = ap.parse_args()
    HCOM = args.hcom
    if args.batch and not re.fullmatch(r"[\w.-]+", args.batch):
        sys.exit("--batch must be a plain name")
    if args.older is not None and (not args.batch or args.older <= 0):
        sys.exit("--older N needs --batch and N > 0")
    OUT.path.mkdir(mode=0o700, parents=True, exist_ok=True)
    if args.batch:
        out_name = f"candidates-{args.batch}.jsonl"
        earlier = [OUT / "candidates.jsonl"] + [OUT / p.name for p in OUT.path.glob("candidates-*.jsonl")]
        used = {c["text"] for path in earlier if path.exists() and path.name != out_name
                for c in read_jsonl(path)}
    elif (OUT / "candidates.jsonl").exists():
        sys.exit("candidates.jsonl exists and is labelled; use --batch NAME for new idle points")
    candidates, skipped = [], collections.Counter()
    if args.older:
        sources = [(f"{inst}@{hashlib.sha1(path.encode()).hexdigest()[:8]}", tool, ["--path", path])
                   for inst, tool, path in older_transcripts()]
    else:
        sources = [(name, tool, [name]) for name, tool in sorted(agents().items())]
    for name, tool, target in sources:
        out = subprocess.run([HCOM, "transcript", *target, "1-", "--json", "--full"],
                             capture_output=True, text=True, timeout=120)
        if out.returncode != 0:
            skipped["no_transcript"] += 1
            continue
        try:
            exchanges = json.loads(out.stdout)
        except json.JSONDecodeError:
            skipped["bad_json"] += 1
            continue
        for cur, nxt in zip(exchanges, exchanges[1:]):
            text = (cur.get("final_text") or cur.get("action") or "").strip()
            if not text:
                skipped["empty_text"] += 1
                continue
            kind = next_kind(nxt.get("user") or "")
            start, end = parse_ts(cur.get("timestamp")), parse_ts(nxt.get("timestamp"))
            ref = f"{name}#{cur['position']}"
            candidates.append({
                # A reused instance name restarts positions, so the id also covers time and text:
                # labels keyed by id never carry over to a different message.
                "id": hashlib.sha1(f"{ref}\n{cur.get('timestamp')}\n{text}".encode()).hexdigest()[:12],
                "cli": tool,
                "lang": "cs" if CS_CHARS.search(text.lower()) else "en",
                "text": text[-MAX_CHARS:],
                "source_field": "final_text" if cur.get("final_text") else "action",
                "next_kind": kind,
                "gap_s": round(end - start) if start and end else None,
                "weak_label": weak_label(text, kind),
                "label": None,
                "source_ref": ref,
            })

    if args.batch:
        new = [c for c in candidates if c["text"] not in used]
        unsampled = len(new)
        if args.older:
            new = random.Random(42).sample(new, min(args.older, len(new)))
        content ="".join(json.dumps(c, ensure_ascii=False) + "\n" for c in new)
        # A batch may already be labelled or used as held-out: it never changes under its name.
        if (OUT / out_name).exists():
            if (OUT / out_name).read_text(encoding="utf-8") != content:
                sys.exit(f"{out_name} exists with different content; use a new --batch name")
        else:
            (OUT / out_name).write_text(content, encoding="utf-8")
        summary = {
            "batch": args.batch, "sources": len(sources), "seen": len(candidates),
            "already_used": len(candidates) - unsampled, "new_unsampled": unsampled,
            "new_candidates": len(new), "skipped": dict(skipped),
            "lang": dict(collections.Counter(c["lang"] for c in new)),
            "cli": dict(collections.Counter(c["cli"] for c in new)),
            "weak_label_x_lang": dict(collections.Counter(f"{c['weak_label']}/{c['lang']}" for c in new)),
        }
        (RESULTS / f"dataset-{args.batch}-summary.json").write_text(json.dumps(summary, indent=2) + "\n")
        print(json.dumps(summary, indent=2))
        return

    rng = random.Random(42)
    cases = []
    for label in LABELS:
        pool = [c for c in candidates if c["weak_label"] == label]
        cases += rng.sample(pool, min(PER_LABEL, len(pool)))

    with open(OUT / "candidates.jsonl", "w", encoding="utf-8") as f:
        f.writelines(json.dumps(c, ensure_ascii=False) + "\n" for c in candidates)
    with open(OUT / "cases.jsonl", "w", encoding="utf-8") as f:
        f.writelines(json.dumps(c, ensure_ascii=False) + "\n" for c in cases)
    summary = {
        "candidates": len(candidates),
        "cases": len(cases),
        "skipped": dict(skipped),
        "weak_label_x_lang": dict(collections.Counter(f"{c['weak_label']}/{c['lang']}" for c in candidates)),
        "next_kind": dict(collections.Counter(c["next_kind"] for c in candidates)),
        "cli": dict(collections.Counter(c["cli"] for c in candidates)),
        "cases_weak_label_x_lang": dict(collections.Counter(f"{c['weak_label']}/{c['lang']}" for c in cases)),
    }
    (RESULTS / "dataset-summary.json").write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(summary, indent=2))


if __name__ == "__main__":
    main()

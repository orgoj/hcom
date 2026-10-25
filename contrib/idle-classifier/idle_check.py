#!/usr/bin/env python3
"""Which idle (listening) hcom agents need a user decision, per local_models' contract v4.

Usage: python3 -I idle_check.py [--json] [--hcom BIN] [--python PY] [--attention FILE] [--head DIR]

Each listening agent's last exchange (`hcom transcript NAME --last 1 --json --detailed`) becomes
the classifier input exactly as final_inputs.py builds it; all inputs go to one
`attention.py score HEAD` call (head.json `default_threshold`, no --threshold). Classes:
needs_user / waiting / done. An agent that left listening, or whose last exchange changed, while
the model ran is reported `stale`, not with a score. Message text is never printed, and failures
report a fixed category and exit code, never command output.
"""
import argparse
import hashlib
import json
import math
import os
import pathlib
import subprocess
import sys

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))  # -I leaves the script directory off sys.path
from final_inputs import final_input  # noqa: E402

CLASSES = ("needs_user", "waiting", "done")
LOCAL_MODELS = pathlib.Path.home() / "work" / "local-models"
HCOM_DIR = pathlib.Path(os.environ.get("HCOM_DIR", pathlib.Path.home() / ".hcom")).expanduser()
DEFAULT_HEAD = HCOM_DIR / "tmp" / "idle-classifier" / "head-3class-xlmr-r5-e21"


class Failure(Exception):
    """The whole command fails; the message is a fixed category, never command output."""


def run(argv, stdin=None):
    """(exit code, stdout, stderr); 127 when the command cannot run or times out."""
    try:
        out = subprocess.run(argv, input=stdin, capture_output=True, text=True, timeout=600)
    except (OSError, subprocess.TimeoutExpired):
        return 127, "", ""
    return out.returncode, out.stdout, out.stderr


def listing(runner, hcom):
    code, out, _ = runner([hcom, "list", "--json"])
    if code != 0:
        raise Failure(f"hcom list failed (exit {code})")
    try:
        agents = json.loads(out)
    except ValueError:
        raise Failure("hcom list output invalid") from None
    if not isinstance(agents, list) or not all(
            isinstance(a, dict) and isinstance(a.get("name"), str) and isinstance(a.get("status"), str)
            for a in agents):
        raise Failure("hcom list output invalid")
    return {a["name"]: a for a in agents}


def last_input(runner, hcom, name):
    """(input_kind, text, exchange identity) of the agent's last exchange, or a reason string."""
    code, out, _ = runner([hcom, "transcript", name, "--last", "1", "--json", "--detailed"])
    if code != 0:
        return "transcript_failed"
    try:
        exchanges = json.loads(out)
    except ValueError:
        return "transcript_invalid"
    if not isinstance(exchanges, list) or not all(isinstance(e, dict) for e in exchanges):
        return "transcript_invalid"
    if not exchanges:
        return "no_exchange"
    ex = exchanges[-1]
    kind, text = final_input(ex)
    if not text:
        return "empty_text"
    return kind, text, (ex.get("position"), ex.get("timestamp"), hashlib.sha1(text.encode()).hexdigest())


def valid_p(value):
    return isinstance(value, (int, float)) and not isinstance(value, bool) and math.isfinite(value) and 0 <= value <= 1


def score(runner, scorer, texts):
    """Scores in input order; ids sent to the scorer are the indices."""
    stdin = "".join(json.dumps({"id": str(i), "text": t}, ensure_ascii=False) + "\n" for i, t in enumerate(texts))
    code, out, _ = runner(scorer, stdin)
    if code != 0:
        raise Failure(f"score failed (exit {code})")
    scores = {}
    for line in out.splitlines():
        if not line.strip():
            continue
        try:
            row = json.loads(line)
        except ValueError:
            raise Failure("score output invalid") from None
        p = row.get("p") if isinstance(row, dict) else None
        if (not isinstance(p, dict) or set(p) != set(CLASSES) or not all(map(valid_p, p.values()))
                or row.get("class") not in CLASSES or not isinstance(row.get("id"), str)):
            raise Failure("score output invalid")
        if row["id"] in scores or row["id"] not in map(str, range(len(texts))):
            raise Failure("score output ids invalid")
        scores[row["id"]] = row
    if len(scores) != len(texts):
        raise Failure("score output ids invalid")
    return [scores[str(i)] for i in range(len(texts))]


def check(runner, hcom, scorer):
    rows, scored = [], []
    for name, agent in listing(runner, hcom).items():
        if agent["status"] != "listening":
            continue
        row = {"name": name, "tool": agent.get("tool"), "class": None, "p_needs_user": None, "p": None,
               "input_kind": None, "status_age_seconds": None, "reason": None}
        got = last_input(runner, hcom, name)
        if isinstance(got, str):
            row["reason"] = got
        else:
            scored.append((row, got))
        rows.append(row)
    if not rows:
        return []
    scores = score(runner, scorer, [text for _, (_, text, _) in scored]) if scored else []
    # The model run takes seconds: keep a score only if the agent is still listening on the same
    # last exchange (an age comparison cannot tell, remote agents always report age 0).
    current = listing(runner, hcom)
    for row in rows:
        agent = current.get(row["name"])
        row["status_age_seconds"] = agent.get("status_age_seconds") if agent else None
        if not agent or agent["status"] != "listening":
            row["reason"] = "stale"
    for (row, (kind, _, identity)), s in zip(scored, scores):
        if row["reason"]:
            continue
        again = last_input(runner, hcom, row["name"])
        if isinstance(again, str) or again[2] != identity:
            row["reason"] = "stale"
            continue
        row.update({"class": s["class"], "p_needs_user": s["p"]["needs_user"], "p": s["p"], "input_kind": kind})
    rank = {c: i for i, c in enumerate(CLASSES)}
    rows.sort(key=lambda r: (rank.get(r["class"], len(CLASSES)),
                             -r["p_needs_user"] if r["p_needs_user"] is not None else math.inf, r["name"]))
    return rows


def table(rows):
    lines = [("NAME", "CLASS", "P_NEEDS_USER")] + [
        (r["name"], r["class"] or f"- ({r['reason']})",
         "-" if r["p_needs_user"] is None else f"{r['p_needs_user']:.3f}") for r in rows]
    widths = [max(len(line[i]) for line in lines) for i in range(2)]
    return "".join(f"{a:<{widths[0]}}  {b:<{widths[1]}}  {c}\n" for a, b, c in lines)


def main(argv=None, runner=run):
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--json", action="store_true")
    ap.add_argument("--hcom", default="hcom")
    ap.add_argument("--python", default=str(LOCAL_MODELS / ".venv" / "bin" / "python"))
    ap.add_argument("--attention", default=str(LOCAL_MODELS / "idle-classifier" / "attention.py"))
    ap.add_argument("--head", default=str(DEFAULT_HEAD))
    args = ap.parse_args(argv)
    try:
        for flag, path in (("--python", args.python), ("--attention", args.attention),
                           ("--head", pathlib.Path(args.head) / "head.json")):
            if not pathlib.Path(path).is_file():
                raise Failure(f"{flag}: {path} not found")
        rows = check(runner, args.hcom, [args.python, args.attention, "score", args.head])
    except Failure as e:
        print(f"idle_check: {e}", file=sys.stderr)
        return 1
    sys.stdout.write(json.dumps(rows, indent=2) + "\n" if args.json else table(rows))
    return 0


if __name__ == "__main__":
    sys.exit(main())

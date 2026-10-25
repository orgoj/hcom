#!/usr/bin/env python3
"""Which idle (listening) hcom agents need a user decision, per local_models' contract v4.

Usage: python3 -I idle_check.py [--json] [--hcom BIN] [--python PY] [--attention FILE] [--head DIR] [--live DIR] [NAME...]
       python3 -I idle_check.py [--live DIR] --label SHA1 CLASS

Each listening agent's (only the NAMEs, when given) last exchange
(`hcom transcript NAME --last 1 --json --detailed`) becomes the classifier input exactly as
final_inputs.py builds it; all inputs go to one `attention.py score HEAD` call (head.json
`default_threshold`, no --threshold). Classes: needs_user / waiting / done. An agent that left listening, or whose last exchange changed, while
the model ran is reported `stale`, not with a score. Message text is never printed, and failures
report a fixed category and exit code, never command output.

Every scored text is kept in the private LIVE/scores.jsonl, which is also the score cache: a text
the same model already scored is not scored again. `--label` records the user's class for a text
(by its text_sha1) in LIVE/corrections.jsonl; the last correction wins over the model's class.
"""
import argparse
import datetime
import fcntl
import hashlib
import json
import math
import os
import pathlib
import re
import subprocess
import sys
from contextlib import contextmanager

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))  # -I leaves the script directory off sys.path
from final_inputs import final_input  # noqa: E402

CLASSES = ("needs_user", "waiting", "done")
LOCAL_MODELS = pathlib.Path.home() / "work" / "local-models"
HCOM_DIR = pathlib.Path(os.environ.get("HCOM_DIR", pathlib.Path.home() / ".hcom")).expanduser()
DEFAULT_HEAD = HCOM_DIR / "tmp" / "idle-classifier" / "head-3class-xlmr-r5-e21"
LIVE_DIR = HCOM_DIR / "tmp" / "idle-classifier" / "live"
SHA1 = re.compile(r"[0-9a-f]{40}")


class Failure(Exception):
    """The whole command fails; the message is a fixed category, never command output."""


def file_sha256(path):
    return hashlib.sha256(pathlib.Path(path).read_bytes()).hexdigest()


HASH_LIMIT = 64 << 20  # larger head files (fine-tuned weights) are identified by file identity


def model_id(attention, head):
    """Cache identity of the scoring model: attention.py and every head file by content; a head file
    over HASH_LIMIT by device, inode, size, mtime and ctime instead (a copy then does not share)."""
    head = pathlib.Path(head)
    parts = [f"attention.py sha256 {file_sha256(attention)}"]
    for f in sorted(head.iterdir()):
        if f.is_file():
            st = f.stat()
            parts.append(f"{f.name} sha256 {file_sha256(f)}" if st.st_size <= HASH_LIMIT else
                         f"{f.name} stat {st.st_dev} {st.st_ino} {st.st_size} {st.st_mtime_ns} {st.st_ctime_ns}")
    return hashlib.sha256("\n".join(parts).encode()).hexdigest()


def now():
    return datetime.datetime.now(datetime.timezone.utc).isoformat(timespec="seconds")


class Store:
    """Private live data (dir 0700, files 0600): scores.jsonl, every scored text and the score cache,
    and corrections.jsonl, the user's classes. Appends run under an exclusive lock on LIVE/.lock;
    lines that are not a valid record (e.g. an interrupted append) are skipped."""

    def __init__(self, path):
        self.path = pathlib.Path(path)

    def _open(self, name, flags):
        fd = os.open(self.path / name, flags | os.O_CREAT, 0o600)
        os.fchmod(fd, 0o600)  # also an existing file
        return fd

    @contextmanager
    def locked(self):
        """Exclusive lock; also tightens the dir and the existing data files, before any read."""
        self.path.mkdir(parents=True, exist_ok=True)
        os.chmod(self.path, 0o700)
        fd = self._open(".lock", os.O_RDWR)
        try:
            fcntl.flock(fd, fcntl.LOCK_EX)
            for name in ("scores.jsonl", "corrections.jsonl"):
                if (self.path / name).exists():
                    os.chmod(self.path / name, 0o600)
            yield
        finally:
            os.close(fd)

    def _rows(self, name):
        try:
            data = (self.path / name).read_text(encoding="utf-8", errors="replace")
        except FileNotFoundError:
            return []
        rows = []
        for line in data.splitlines():
            try:
                row = json.loads(line)
            except ValueError:
                continue
            if isinstance(row, dict) and isinstance(row.get("text_sha1"), str) and SHA1.fullmatch(row["text_sha1"]):
                rows.append(row)
        return rows

    def _append(self, name, rows):
        """Complete lines in one write; a file not ending in a newline first gets one, so a torn
        earlier append stays a single skipped line. The caller holds the lock."""
        data = "".join(json.dumps(r, ensure_ascii=False) + "\n" for r in rows).encode()
        fd = self._open(name, os.O_RDWR | os.O_APPEND)
        try:
            size = os.fstat(fd).st_size
            if size and os.pread(fd, 1, size - 1) != b"\n":
                data = b"\n" + data
            view = memoryview(data)
            while view:
                view = view[os.write(fd, view):]
        finally:
            os.close(fd)

    def scores(self, model):
        """{text_sha1: score row} of the valid rows this model scored."""
        return {r["text_sha1"]: r for r in self._rows("scores.jsonl") if r.get("model") == model and valid_score(r)}

    def add_scores(self, model, rows):
        """Append the rows whose text this model has no score for yet (another run may have added it)."""
        with self.locked():
            have = self.scores(model)
            new = [r for r in rows if r["text_sha1"] not in have]
            if new:
                self._append("scores.jsonl", new)

    def corrections(self):
        """{text_sha1: class}, the last valid correction of each text."""
        return {r["text_sha1"]: r["class"] for r in self._rows("corrections.jsonl") if r.get("class") in CLASSES}

    def label(self, sha, cls):
        with self.locked():
            self._append("corrections.jsonl", [{"ts": now(), "text_sha1": sha, "class": cls}])


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


def valid_score(row):
    p = row.get("p") if isinstance(row, dict) else None
    return (isinstance(p, dict) and set(p) == set(CLASSES) and all(map(valid_p, p.values()))
            and row.get("class") in CLASSES)


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
        if not valid_score(row) or not isinstance(row.get("id"), str):
            raise Failure("score output invalid")
        if row["id"] in scores or row["id"] not in map(str, range(len(texts))):
            raise Failure("score output ids invalid")
        scores[row["id"]] = row
    if len(scores) != len(texts):
        raise Failure("score output ids invalid")
    return [scores[str(i)] for i in range(len(texts))]


def check(runner, hcom, scorer, names=None, store=None, model=None, head=None):
    """Without a store nothing is cached or logged and no correction applies."""
    rows, scored = [], []
    for name, agent in listing(runner, hcom).items():
        if agent["status"] != "listening" or (names and name not in names):
            continue
        row = {"name": name, "tool": agent.get("tool"), "class": None, "p_needs_user": None, "p": None,
               "input_kind": None, "status_age_seconds": None, "reason": None, "text_sha1": None,
               "corrected": False}
        got = last_input(runner, hcom, name)
        if isinstance(got, str):
            row["reason"] = got
        else:
            scored.append((row, got))
        rows.append(row)
    if not rows:
        return []
    cached, corrected = {}, {}
    try:
        if store:
            with store.locked():
                cached, corrected = store.scores(model), store.corrections()
    except OSError:
        raise Failure("live data unreadable") from None
    # One scorer input per text not cached yet, in agent order.
    todo = {}
    for row, (kind, text, (_, _, sha)) in scored:
        if sha not in cached and sha not in todo:
            todo[sha] = (row, kind, text)
    fresh = dict(zip(todo, score(runner, scorer, [text for _, _, text in todo.values()]))) if todo else {}
    if store and fresh:
        try:
            store.add_scores(model, [{"ts": now(), "name": row["name"], "tool": row["tool"], "input_kind": kind,
                                      "text_sha1": sha, "text": text, "model": model, "head": head,
                                      "p": fresh[sha]["p"], "class": fresh[sha]["class"]}
                                     for sha, (row, kind, text) in todo.items()])
        except OSError:
            raise Failure("live data write failed") from None
    scores = [fresh.get(sha) or cached[sha] for _, (_, _, (_, _, sha)) in scored]
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
        sha = identity[2]
        row.update({"class": corrected.get(sha, s["class"]), "p_needs_user": s["p"]["needs_user"], "p": s["p"],
                    "input_kind": kind, "text_sha1": sha, "corrected": sha in corrected})
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
    ap.add_argument("--live", default=str(LIVE_DIR), help="private score log, cache and corrections")
    ap.add_argument("--label", nargs=2, metavar=("SHA1", "CLASS"), help="record the user's class for a text")
    ap.add_argument("names", nargs="*", metavar="NAME", help="check only these agents (default: all)")
    args = ap.parse_args(argv)
    store = Store(args.live)
    try:
        if args.label:  # needs no model
            sha, cls = args.label
            if not SHA1.fullmatch(sha) or cls not in CLASSES:
                raise Failure("--label: needs a text_sha1 and one of " + ", ".join(CLASSES))
            try:
                store.label(sha, cls)
            except OSError:
                raise Failure("live data write failed") from None
            return 0
        for flag, path in (("--python", args.python), ("--attention", args.attention),
                           ("--head", pathlib.Path(args.head) / "head.json")):
            if not pathlib.Path(path).is_file():
                raise Failure(f"{flag}: {path} not found")
        head = pathlib.Path(args.head).resolve()
        try:
            model = model_id(args.attention, head)
        except OSError:
            raise Failure("model files unreadable") from None
        rows = check(runner, args.hcom, [args.python, args.attention, "score", args.head], set(args.names),
                     store, model, str(head))
    except Failure as e:
        print(f"idle_check: {e}", file=sys.stderr)
        return 1
    sys.stdout.write(json.dumps(rows, indent=2) + "\n" if args.json else table(rows))
    return 0


if __name__ == "__main__":
    sys.exit(main())

#!/usr/bin/env python3
"""Independent labels from Claude Haiku or Sonnet (user-approved labelling; texts go to the cloud).

Usage: python3 -I label_haiku.py [--rules v1|v2] [--model haiku|sonnet] [--source STEM] [--ids FILE]
                                 [--max-batches N]
  v1: contract_v1 rules, 4-way state, haiku           -> haiku_labels.jsonl (cases only)
  v2: contract_v2 rules, must_act (needs_user yes/no) + state -> <model>_v2_<source>-<rules hash>.jsonl
The model sees only the message text and the rules (blind to any other label or score).
--source is any private JSONL with id and text; --ids limits the run to the listed ids.

Labels go to the private data directory, per-call metrics to results/<stem>-calls.jsonl and a
run summary to results/<stem>-summary.json. Already labelled ids are skipped, so a run resumes
and nothing is labelled twice; --max-batches 1 is the one-call verification and is simply the
first batch of the run. Each label row records the hash of the rules that produced it; v2
file names carry it too, so changed rules start a new set. Duplicate ids are refused, an id
answered twice in one response is dropped and retried, and the summary counts only ids of the
current source file.
"""
import argparse
import hashlib
import json
import os
import pathlib
import re
import statistics
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
STATES = ["done", "needs_user", "blocked", "waiting_agent"]
BATCH = 10
BATCH_NOTE = ("You get several messages, each in <message id=\"...\"> tags. Label every message "
              "independently and return one entry per id.")


def rules_and_schema(version):
    contract = json.loads((HERE / f"contract_{version}.json").read_text())
    rules = contract["request"]["messages"][0]["content"].rsplit("Answer only with JSON", 1)[0] + BATCH_NOTE
    item = {"id": {"type": "string"}, "state": {"type": "string", "enum": STATES}}
    if version == "v2":
        item["needs_user"] = {"type": "string", "enum": ["yes", "no"]}
    schema = {"type": "object", "required": ["labels"], "properties": {"labels": {"type": "array", "items": {
        "type": "object", "properties": item, "required": list(item)}}}}
    return rules, schema


def read_jsonl(path):
    if not path.exists():
        return []
    with open(path, encoding="utf-8") as f:
        return [json.loads(line) for line in f if line.strip()]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--rules", choices=["v1", "v2"], default="v1")
    ap.add_argument("--model", choices=["haiku", "sonnet"], default="haiku")
    ap.add_argument("--source", default="cases",
                    help="private JSONL with id and text: cases, candidates[-<batch>], labels-...")
    ap.add_argument("--ids", help="private file with one id per line: label only these source rows")
    ap.add_argument("--max-batches", type=int, default=0, help="0 = all")
    args = ap.parse_args()
    if not re.fullmatch(r"[\w.-]+", args.source):
        sys.exit("--source must be a plain file stem in the private data directory")
    if args.rules == "v1" and (args.source != "cases" or args.model != "haiku"):
        sys.exit("v1 labels exist for cases with haiku only")
    rules, schema = rules_and_schema(args.rules)
    rules_hash = hashlib.sha1(rules.encode()).hexdigest()[:8]
    # v2 files carry the rules hash, so changed rules start a new label set instead of mixing.
    stem = "haiku_labels" if args.rules == "v1" else f"{args.model}_v2_{args.source}-{rules_hash}"

    out_path, calls_path = OUT / f"{stem}.jsonl", RESULTS / f"{stem}-calls.jsonl"
    existing = read_jsonl(out_path)
    if any(r.get("rules", rules_hash) != rules_hash for r in existing):
        sys.exit(f"{out_path.name} was labelled with other rules")
    done = {r["id"] for r in existing}
    if len(done) != len(existing):
        sys.exit(f"{out_path.name} has duplicate ids")
    cases = read_jsonl(OUT / f"{args.source}.jsonl")
    if any(not isinstance(c.get("id"), str) or not isinstance(c.get("text"), str) for c in cases):
        sys.exit(f"{args.source}.jsonl needs string id and text in every row")
    if len({c["id"] for c in cases}) != len(cases):
        sys.exit(f"{args.source}.jsonl has duplicate ids")
    if args.ids:
        wanted = set((OUT / args.ids).read_text(encoding="utf-8").split())
        if not wanted <= {c["id"] for c in cases}:
            sys.exit(f"{args.ids} has ids that are not in {args.source}.jsonl")
        cases = [c for c in cases if c["id"] in wanted]
    source_ids = {c["id"] for c in cases}
    todo = [c for c in cases if c["id"] not in done]
    if args.max_batches > 0:
        todo = todo[:args.max_batches * BATCH]

    for i in range(0, len(todo), BATCH):
        batch = todo[i:i + BATCH]
        prompt = "\n\n".join(f'<message id="{c["id"]}">\n{c["text"]}\n</message>' for c in batch)
        proc = subprocess.run(
            ["claude", "-p", "--model", args.model, "--tools", "", "--setting-sources", "",
             "--output-format", "json", "--json-schema", json.dumps(schema), "--system-prompt", rules],
            input=prompt, capture_output=True, text=True, timeout=300,
        )
        try:
            # --output-format json prints an array of events; the last `result` event carries the answer.
            result = [e for e in json.loads(proc.stdout) if e.get("type") == "result"][-1]
            if result.get("is_error"):
                raise ValueError(result.get("api_error_status"))
            labels = result["structured_output"]["labels"]
        except (json.JSONDecodeError, KeyError, TypeError, IndexError, ValueError) as e:
            # stderr and the exception text may quote message texts: report the category only.
            sys.exit(f"batch {i // BATCH}: unusable output ({type(e).__name__}, exit {proc.returncode})")
        ids = {c["id"] for c in batch}
        valid = [x for x in labels if x.get("id") in ids and x.get("state") in STATES
                 and (args.rules == "v1" or x.get("needs_user") in ("yes", "no"))]
        # An id answered more than once is ambiguous: drop it, a later run retries it.
        seen = [x["id"] for x in valid]
        valid = [x for x in valid if seen.count(x["id"]) == 1]
        usage = result.get("usage") or {}
        with calls_path.open("a", encoding="utf-8") as f:
            f.write(json.dumps({
                "rules": rules_hash, "cases": len(batch), "labels": len(valid),
                "models": list(result.get("modelUsage") or {}),
                "duration_ms": result.get("duration_ms"), "duration_api_ms": result.get("duration_api_ms"),
                "ttft_ms": result.get("ttft_ms"), "cost_usd": result.get("total_cost_usd"),
                "input_tokens": sum(usage.get(k) or 0 for k in (
                    "input_tokens", "cache_creation_input_tokens", "cache_read_input_tokens")),
                "output_tokens": usage.get("output_tokens"),
            }) + "\n")
        with out_path.open("a", encoding="utf-8") as f:
            for x in valid:
                row = {"id": x["id"], "haiku": x["state"], "rules": rules_hash}
                if args.rules == "v2":
                    row["must_act"] = x["needs_user"] == "yes"
                f.write(json.dumps(row) + "\n")
        print(f"batch {i // BATCH}: {len(valid)}/{len(batch)} labels")

    calls = [c for c in read_jsonl(calls_path) if c.get("rules", rules_hash) == rules_hash]
    labelled = [r for r in read_jsonl(out_path) if r["id"] in source_ids]  # current source only
    durations = [c["duration_ms"] for c in calls if c.get("duration_ms") is not None]
    summary = {
        "stem": stem, "rules": rules_hash, "source_cases": len(cases), "labelled": len({r["id"] for r in labelled}),
        "calls": len(calls), "cost_usd": round(sum(c.get("cost_usd") or 0 for c in calls), 4),
        "duration_ms_per_call": {"p50": statistics.median(durations), "max": max(durations)} if durations else None,
        "ms_per_case": round(sum(durations) / sum(c["cases"] for c in calls)) if durations else None,
        "input_tokens": sum(c.get("input_tokens") or 0 for c in calls),
        "output_tokens": sum(c.get("output_tokens") or 0 for c in calls),
        "states": {s: sum(r["haiku"] == s for r in labelled) for s in STATES},
    }
    if args.rules == "v2":
        summary["must_act"] = sum(r["must_act"] for r in labelled)
    (RESULTS / f"{stem}-summary.json").write_text(json.dumps(summary, indent=2) + "\n")
    print(json.dumps(summary))


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Second, independent label for cases.jsonl from Claude Haiku (user-approved labelling step).

Usage: python3 -I label_haiku.py [MAX_BATCHES [OUT_STEM]]   (verify with 1 before a full run; 0 = all)
MAX_BATCHES > 0 is a verification run: it writes <OUT_STEM>-probe files to the private data
directory only.
Sends batches of cases to `claude -p --model haiku` with the contract's label rules and writes
<OUT_STEM>.jsonl ({id, haiku}, default haiku_labels) to the private data directory and per-call
metrics to results/<OUT_STEM>-calls.jsonl.
Already labelled ids are skipped, so a rerun resumes.
"""
import json
import os
import pathlib
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
CONTRACT = HERE / "contract_v1.json"
LABELS = ["done", "needs_user", "blocked", "waiting_agent"]
BATCH = 10

RULES = json.loads(CONTRACT.read_text())["request"]["messages"][0]["content"]
RULES = RULES.rsplit("Answer only with JSON", 1)[0] + (
    "You get several messages, each in <message id=\"...\"> tags. Label every message "
    "independently and return one entry per id."
)
SCHEMA = {
    "type": "object",
    "properties": {"labels": {"type": "array", "items": {
        "type": "object",
        "properties": {"id": {"type": "string"}, "state": {"type": "string", "enum": LABELS}},
        "required": ["id", "state"],
    }}},
    "required": ["labels"],
}


def main():
    cases = [json.loads(line) for line in (OUT / "cases.jsonl").open(encoding="utf-8")]
    stem = sys.argv[2] if len(sys.argv) > 2 else "haiku_labels"
    probe = len(sys.argv) > 1 and int(sys.argv[1]) > 0
    if probe:  # a verification run never touches full-set files and starts from the first batch
        stem += "-probe"
        for path in (OUT / f"{stem}.jsonl", OUT / f"{stem}-calls.jsonl"):
            path.unlink(missing_ok=True)
    out_path = OUT / f"{stem}.jsonl"
    calls_path = (OUT if probe else RESULTS) / f"{stem}-calls.jsonl"
    done = set()
    if out_path.exists():
        done = {json.loads(line)["id"] for line in out_path.open(encoding="utf-8")}
    todo = [c for c in cases if c["id"] not in done]
    if probe:
        todo = todo[:int(sys.argv[1]) * BATCH]
    for i in range(0, len(todo), BATCH):
        batch = todo[i:i + BATCH]
        prompt = "\n\n".join(f'<message id="{c["id"]}">\n{c["text"]}\n</message>' for c in batch)
        proc = subprocess.run(
            ["claude", "-p", "--model", "haiku", "--tools", "", "--setting-sources", "",
             "--output-format", "json", "--json-schema", json.dumps(SCHEMA),
             "--system-prompt", RULES],
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
        usage = result.get("usage") or {}
        with calls_path.open("a", encoding="utf-8") as f:
            f.write(json.dumps({
                "cases": len(batch), "labels": len(labels), "models": list(result.get("modelUsage") or {}),
                "duration_ms": result.get("duration_ms"), "duration_api_ms": result.get("duration_api_ms"),
                "ttft_ms": result.get("ttft_ms"), "cost_usd": result.get("total_cost_usd"),
                "input_tokens": sum(usage.get(k) or 0 for k in (
                    "input_tokens", "cache_creation_input_tokens", "cache_read_input_tokens")),
                "output_tokens": usage.get("output_tokens"),
            }) + "\n")
        with out_path.open("a", encoding="utf-8") as f:
            for item in labels:
                if item.get("id") in ids and item.get("state") in LABELS:
                    f.write(json.dumps({"id": item["id"], "haiku": item["state"]}) + "\n")
        print(f"batch {i // BATCH}: {len(labels)} labels")


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Labels and held-out check for local_models' contract v2 (`attention.py`, encoder + head).

Usage:
  python3 -I attention_data.py export          # labels-train.jsonl, labels-holdout.jsonl
  python3 -I attention_data.py check SCORES    # SCORES = attention.py score output on the holdout

must_act = gold label in {needs_user, blocked}; gold = manual `label` when set, else Haiku.
A stratified 20 % (by must_act and lang, seed 42) is held out from training for the final
threshold check. Texts and splits stay in the private data directory; the check summary goes to
results/attention-holdout-summary.json.
"""
import json
import os
import pathlib
import random
import sys

HERE = pathlib.Path(__file__).resolve().parent
RESULTS = HERE / "results"  # metrics only, committed
ATTENTION = {"needs_user", "blocked"}
HOLDOUT_SHARE = 0.2


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


def write_jsonl(path, rows):
    with open(path, "w", encoding="utf-8") as f:
        f.writelines(json.dumps(r, ensure_ascii=False) + "\n" for r in rows)


def export():
    haiku = {r["id"]: r["haiku"] for r in read_jsonl(OUT / "haiku_labels.jsonl")}
    rows = []
    for c in read_jsonl(OUT / "cases.jsonl"):
        gold = c.get("label") or haiku.get(c["id"])
        if gold:
            rows.append({"id": c["id"], "text": c["text"], "must_act": gold in ATTENTION, "lang": c["lang"]})
    rng = random.Random(42)
    train, holdout = [], []
    for key in sorted({(r["must_act"], r["lang"]) for r in rows}):
        group = [r for r in rows if (r["must_act"], r["lang"]) == key]
        rng.shuffle(group)
        cut = round(len(group) * HOLDOUT_SHARE)
        holdout += group[:cut]
        train += group[cut:]
    write_jsonl(OUT / "labels-train.jsonl", train)
    write_jsonl(OUT / "labels-holdout.jsonl", holdout)
    count = lambda rs: {"n": len(rs), "must_act": sum(r["must_act"] for r in rs),
                        "cs": sum(r["lang"] == "cs" for r in rs)}
    print(json.dumps({"train": count(train), "holdout": count(holdout)}))


def check(scores_path):
    gold = {r["id"]: r["must_act"] for r in read_jsonl(OUT / "labels-holdout.jsonl")}
    rows = read_jsonl(scores_path)
    scores = {r["id"]: r for r in rows}
    # Metrics are only meaningful for exactly the holdout, scored once, with boolean decisions.
    if len(scores) != len(rows) or set(scores) != set(gold):
        sys.exit("SCORES must hold each holdout id exactly once (duplicate, missing or extra ids)")
    if any(not isinstance(r.get("must_act"), bool) for r in rows):
        sys.exit("SCORES must_act must be a boolean in every row")
    att = [i for i in gold if gold[i]]
    other = [i for i in gold if not gold[i]]
    missed = [i for i in att if not scores[i]["must_act"]]
    false_hl = [i for i in other if scores[i]["must_act"]]
    lat = sorted(s["latency_ms"] for s in scores.values() if s.get("latency_ms") is not None)
    summary = {
        "holdout": len(gold), "attention": len(att), "other": len(other),
        "missed": f"{len(missed)}/{len(att)}", "false_highlight": f"{len(false_hl)}/{len(other)}",
        "missed_ids": missed, "false_highlight_ids": false_hl,
        "lowest_p_on_attention": min((scores[i]["p_must_act"] for i in att), default=None),
        "latency_ms": {"p50": lat[len(lat) // 2], "max": lat[-1]} if lat else None,
    }
    (RESULTS / "attention-holdout-summary.json").write_text(json.dumps(summary, indent=2) + "\n")
    print(json.dumps(summary, indent=2))


if __name__ == "__main__":
    if sys.argv[1:2] == ["export"]:
        export()
    elif sys.argv[1:2] == ["check"] and len(sys.argv) == 3:
        check(sys.argv[2])
    else:
        sys.exit(__doc__)

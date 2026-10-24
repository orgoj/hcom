#!/usr/bin/env python3
"""Labels and held-out check for local_models' contract v2 (`attention.py`, encoder + head).

Usage:
  python3 -I attention_data.py export LABELS          # labels-train<sfx>.jsonl, labels-holdout<sfx>.jsonl
  python3 -I attention_data.py check LABELS SCORES    # SCORES = attention.py score output on the holdout

LABELS v1: the 174 cases, must_act = Haiku v1 state in {needs_user, blocked} (no suffix).
LABELS v2: all candidates, must_act from Haiku with contract v2 rules (suffix -v2).
A manual `label` on a case overrides Haiku. A stratified 20 % (by must_act and lang, seed 42)
is held out from training for the final threshold check. Texts and splits stay in the private
data directory; the check summary goes to results/attention-holdout<sfx>-summary.json.
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


SUFFIX = {"v1": "", "v2": "-v2"}
HAIKU_V2_RULES = "6d617f6c"  # rules hash of the Haiku v2 run the v2 labels come from


def gold_rows(labels):
    if labels == "v1":
        source, haiku = "cases.jsonl", {r["id"]: r["haiku"] in ATTENTION for r in read_jsonl(OUT / "haiku_labels.jsonl")}
    else:
        source = "candidates.jsonl"
        haiku = {r["id"]: r["must_act"] for r in read_jsonl(OUT / f"haiku_v2_candidates-{HAIKU_V2_RULES}.jsonl")}
    # Manual review lives in cases.jsonl (a sampled copy), so it applies to candidates by id too.
    manual = {c["id"]: c["label"] for c in read_jsonl(OUT / "cases.jsonl") if c.get("label")}
    rows = []
    for c in read_jsonl(OUT / source):
        label = c.get("label") or manual.get(c["id"])
        must_act = label in ATTENTION if label else haiku.get(c["id"])
        if must_act is not None:
            rows.append({"id": c["id"], "text": c["text"], "must_act": must_act, "lang": c["lang"]})
    return rows


def export(labels):
    rows = gold_rows(labels)
    rng = random.Random(42)
    train, holdout = [], []
    for key in sorted({(r["must_act"], r["lang"]) for r in rows}):
        group = [r for r in rows if (r["must_act"], r["lang"]) == key]
        rng.shuffle(group)
        cut = round(len(group) * HOLDOUT_SHARE)
        holdout += group[:cut]
        train += group[cut:]
    write_jsonl(OUT / f"labels-train{SUFFIX[labels]}.jsonl", train)
    write_jsonl(OUT / f"labels-holdout{SUFFIX[labels]}.jsonl", holdout)
    count = lambda rs: {"n": len(rs), "must_act": sum(r["must_act"] for r in rs),
                        "cs": sum(r["lang"] == "cs" for r in rs)}
    print(json.dumps({"train": count(train), "holdout": count(holdout)}))


def check(labels, scores_path):
    gold = {r["id"]: r["must_act"] for r in read_jsonl(OUT / f"labels-holdout{SUFFIX[labels]}.jsonl")}
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
    (RESULTS / f"attention-holdout{SUFFIX[labels]}-summary.json").write_text(json.dumps(summary, indent=2) + "\n")
    print(json.dumps(summary, indent=2))


if __name__ == "__main__":
    if sys.argv[1:2] == ["export"] and len(sys.argv) == 3 and sys.argv[2] in SUFFIX:
        export(sys.argv[2])
    elif sys.argv[1:2] == ["check"] and len(sys.argv) == 4 and sys.argv[2] in SUFFIX:
        check(sys.argv[2], sys.argv[3])
    else:
        sys.exit(__doc__)

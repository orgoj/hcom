#!/usr/bin/env python3
"""Compare a labels file with a second, blind labelling and write a corrected copy.

Usage: python3 -I compare_labels.py BASE SECOND OUT [--ids FILE]
  BASE    private labels JSONL stem with id, text, must_act, lang (e.g. labels-fresh-2026-10-24b)
  SECOND  private label stem from label_haiku.py with id, must_act (e.g. sonnet_v2_...-6d617f6c)
  OUT     new private stem: BASE rows with must_act taken from SECOND wherever SECOND has the id
  --ids   private file with one id per line: use SECOND only for these ids (each must be in it)

BASE is never modified; OUT is written once (an identical rebuild is a no-op, anything else is
refused). The comparison (counts, agreement, disagreeing ids, agreement with hcom_dev's
manual_must_act.jsonl corrections) goes to results/compare-OUT.json.
"""
import collections
import json
import os
import pathlib
import re
import sys

HERE = pathlib.Path(__file__).resolve().parent
RESULTS = HERE / "results"  # metrics and ids only, committed


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


def by_id(rows, name):
    out = {r["id"]: r for r in rows}
    if len(out) != len(rows):
        sys.exit(f"{name} has duplicate ids")
    if any(not isinstance(r.get("must_act"), bool) for r in rows):
        sys.exit(f"{name} needs boolean must_act in every row")
    return out


def main():
    args = sys.argv[1:]
    ids_name = None
    if len(args) == 5 and args[3] == "--ids":
        ids_name = args.pop(4)
        args.pop(3)
    if len(args) != 3 or not all(re.fullmatch(r"[\w.-]+", a) for a in args + [ids_name or "x"]):
        sys.exit(__doc__)
    base_name, second_name, out_name = args
    if out_name in (base_name, second_name):
        sys.exit("OUT must be a new name")
    base_rows = read_jsonl(OUT / f"{base_name}.jsonl")
    base = by_id(base_rows, base_name)
    second = by_id(read_jsonl(OUT / f"{second_name}.jsonl"), second_name)
    if ids_name:
        wanted = set((OUT / ids_name).read_text(encoding="utf-8").split())
        if not wanted <= set(second):
            sys.exit(f"{ids_name} has ids that {second_name} did not label")
        second = {i: r for i, r in second.items() if i in wanted}
    if not set(second) <= set(base):
        sys.exit(f"compared ids of {second_name} are not all in {base_name}")
    manual = None
    manual_path = OUT / "manual_must_act.jsonl"
    if manual_path.exists():  # validated before any output is written
        manual = {i: r["must_act"] for i, r in by_id(read_jsonl(manual_path), manual_path.name).items()}

    corrected = [dict(r, must_act=second[r["id"]]["must_act"]) if r["id"] in second else r for r in base_rows]
    content = "".join(json.dumps(r, ensure_ascii=False) + "\n" for r in corrected)
    path = OUT / f"{out_name}.jsonl"
    if path.exists():
        if path.read_text(encoding="utf-8") != content:
            sys.exit(f"{path.name} exists with different content; use a new OUT name")
    else:
        path.write_text(content, encoding="utf-8")

    pairs = collections.Counter(f"{base[i]['must_act']}->{second[i]['must_act']}" for i in second)
    by_lang = collections.Counter(
        f"{base[i]['lang']}:{'agree' if base[i]['must_act'] == second[i]['must_act'] else 'disagree'}" for i in second)
    summary = {
        "base": base_name, "second": second_name, "out": out_name,
        "compared": len(second), "agree": sum(base[i]["must_act"] == second[i]["must_act"] for i in second),
        "base_to_second": dict(pairs), "by_lang": dict(by_lang),
        "must_act": {"base": sum(r["must_act"] for r in base_rows), "out": sum(r["must_act"] for r in corrected)},
        "disagreeing_ids": sorted(i for i in second if base[i]["must_act"] != second[i]["must_act"]),
    }
    if manual is not None:
        shared = [i for i in second if i in manual]
        summary["manual_corrections_compared"] = len(shared)
        summary["manual_corrections_agree"] = sum(manual[i] == second[i]["must_act"] for i in shared)
    (RESULTS / f"compare-{out_name}.json").write_text(json.dumps(summary, indent=2) + "\n")
    print(json.dumps({k: v for k, v in summary.items() if k != "disagreeing_ids"}, indent=2))


if __name__ == "__main__":
    main()

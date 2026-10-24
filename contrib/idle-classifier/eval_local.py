#!/usr/bin/env python3
"""Run the local classifier on cases.jsonl through contract_v1.json (4 labels) and score it.

Usage: python3 -I eval_local.py [MODEL [MAX_CASES]]   (default qwen3.5:4b; verify with 1 first)
Gold per case: manual `label` when set, else the Haiku label (haiku_labels.jsonl).
Writes contract_v1_<model>-runs.jsonl and review.jsonl (cases needing a manual look) to the private data
directory, results/contract_v1_<model>-summary.json here.
Metric definitions follow local_models' TEST-CONCEPT.md.
"""
import collections
import hashlib
import json
import math
import os
import pathlib
import re
import statistics
import sys
import time
import urllib.error
import urllib.request

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
CONTRACT = json.loads((HERE / "contract_v1.json").read_text())
MODEL = sys.argv[1] if len(sys.argv) > 1 else "qwen3.5:4b"
# One safe filename component; the digest keeps distinct tags (a/b:c, a_b:c) apart.
MODEL_STEM = re.sub(r"[^\w.-]", "_", MODEL)
if MODEL_STEM != MODEL:
    MODEL_STEM += "-" + hashlib.sha1(MODEL.encode()).hexdigest()[:8]
DIGESTS = {"qwen3.5:4b": "2a654d98e6fba55d452b7043684e9b57a947e393bbffa62485a7aac05ee4eefd"}
BASE = "http://127.0.0.1:11434"
LABELS = ["done", "needs_user", "blocked", "waiting_agent"]
ATTENTION = {"needs_user", "blocked"}
STATE_PREFIX = re.compile(r'"state"\s*:\s*"$')
THRESHOLDS = [0.0, 0.5, 0.7, 0.8, 0.9, 0.95]


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, *args, **kwargs):
        return None  # a redirect becomes an HTTP error, never a request elsewhere


# Message texts go only to the local Ollama: no HTTP(S)_PROXY, no redirects.
LOCAL = urllib.request.build_opener(urllib.request.ProxyHandler({}), NoRedirect)


def api(path, body=None, timeout=CONTRACT["client_timeout_s"]):
    data = json.dumps(body).encode() if body is not None else None
    req = urllib.request.Request(BASE + path, data=data, headers={"Content-Type": "application/json"})
    with LOCAL.open(req, timeout=timeout) as resp:
        return json.loads(resp.read())


def confidence(logprobs):
    acc = ""
    for tok in logprobs or []:
        if STATE_PREFIX.search(acc):
            return math.exp(tok["logprob"])
        acc += tok["token"]
    return None


def classify(text):
    req = json.loads(json.dumps(CONTRACT["request"]))
    req["model"] = MODEL
    message = text[-CONTRACT["input_max_chars"]:]
    req["messages"][1]["content"] = CONTRACT["user_message_template"].format(message=message)
    t0 = time.perf_counter()
    try:
        resp = api("/api/chat", req)
    except (urllib.error.URLError, TimeoutError) as e:
        return {"state": None, "conf": None, "wall_s": time.perf_counter() - t0, "raw": f"error: {e}"}
    wall = time.perf_counter() - t0
    content = resp["message"]["content"]
    try:
        state = json.loads(content).get("state")
    except (json.JSONDecodeError, AttributeError):
        state = None
    return {"state": state if state in LABELS else None, "conf": confidence(resp.get("logprobs")),
            "wall_s": wall, "raw": content}


def pct(values, q):
    values = sorted(values)
    return values[min(len(values) - 1, int(round(q * (len(values) - 1))))] if values else None


def main():
    tags = {m["name"]: m["digest"] for m in api("/api/tags")["models"]}
    if MODEL in DIGESTS and tags.get(MODEL) != DIGESTS[MODEL]:
        sys.exit(f"{MODEL}: digest {tags.get(MODEL)} differs from the handover pin")
    cases = [json.loads(line) for line in (OUT / "cases.jsonl").open(encoding="utf-8")]
    probe = len(sys.argv) > 2  # a verification run must not replace full-set files
    if probe:
        cases = cases[:int(sys.argv[2])]
    haiku = {}
    if (OUT / "haiku_labels.jsonl").exists():
        haiku = {r["id"]: r["haiku"] for r in map(json.loads, (OUT / "haiku_labels.jsonl").open(encoding="utf-8"))}

    runs = []
    for c in cases:
        first, second = classify(c["text"]), classify(c["text"])
        gold = c.get("label") or haiku.get(c["id"])
        runs.append({"id": c["id"], "lang": c["lang"], "cli": c["cli"], "weak": c["weak_label"],
                     "haiku": haiku.get(c["id"]), "gold": gold, "gold_src": "manual" if c.get("label") else "haiku",
                     "pred": first["state"], "conf": first["conf"], "wall_s": first["wall_s"],
                     "deterministic": first["raw"] == second["raw"]})
    stem = f"contract_v1_{MODEL_STEM}"
    with open(OUT / f"{stem}-runs{'-probe' if probe else ''}.jsonl", "w", encoding="utf-8") as f:
        f.writelines(json.dumps(r) + "\n" for r in runs)

    scored = [r for r in runs if r["gold"]]
    acc = lambda rs: round(sum(r["pred"] == r["gold"] for r in rs) / len(rs), 3) if rs else None
    confusion = collections.defaultdict(collections.Counter)
    for r in scored:
        confusion[r["gold"]][r["pred"] or "invalid"] += 1
    gold_att = [r for r in scored if r["gold"] in ATTENTION]
    gold_other = [r for r in scored if r["gold"] not in ATTENTION]
    sweep = []
    for t in THRESHOLDS:
        kept = [r for r in scored if r["pred"] and (r["conf"] or 0) >= t]
        sweep.append({"threshold": t, "coverage": round(len(kept) / len(scored), 3), "accuracy_kept": acc(kept),
                      "wrong_kept": sum(r["pred"] != r["gold"] for r in kept),
                      "missed_attention_kept": sum(r["gold"] in ATTENTION and r["pred"] not in ATTENTION for r in kept)})
    agreed = [r for r in scored if r["weak"] == r["haiku"]]
    walls = [r["wall_s"] for r in runs]
    summary = {
        "model": MODEL, "cases": len(runs), "scored": len(scored),
        "gold_counts": dict(collections.Counter(r["gold"] for r in scored)),
        "gold_source": dict(collections.Counter(r["gold_src"] for r in scored)),
        "accuracy": {"all": acc(scored), "cs": acc([r for r in scored if r["lang"] == "cs"]),
                     "en": acc([r for r in scored if r["lang"] == "en"]),
                     "regex_and_haiku_agree": acc(agreed), "agree_n": len(agreed)},
        "regex_vs_haiku_agreement": acc([{"pred": r["weak"], "gold": r["haiku"]} for r in scored]),
        "confusion_gold_x_pred": {g: dict(confusion[g]) for g in LABELS},
        "missed_attention": f"{sum(r['pred'] not in ATTENTION for r in gold_att)}/{len(gold_att)}",
        "false_highlight": f"{sum(r['pred'] in ATTENTION for r in gold_other)}/{len(gold_other)}",
        "invalid": sum(r["pred"] is None for r in runs),
        "deterministic": all(r["deterministic"] for r in runs),
        "latency_s": {"p50": round(statistics.median(walls), 3), "p95": round(pct(walls, 0.95), 3),
                      "max": round(max(walls), 3)},
        "threshold_sweep": sweep,
    }
    review = [dict(c, haiku=haiku.get(c["id"]), local=r["pred"]) for c, r in zip(cases, runs)
              if not c.get("label") and len({c["weak_label"], haiku.get(c["id"]), r["pred"]}) > 1]
    summary["review_cases"] = len(review)
    if not probe:
        (RESULTS / f"{stem}-summary.json").write_text(json.dumps(summary, indent=2) + "\n")
        with open(OUT / "review.jsonl", "w", encoding="utf-8") as f:
            f.writelines(json.dumps(c, ensure_ascii=False) + "\n" for c in review)
    print(json.dumps(summary, indent=2))


if __name__ == "__main__":
    main()

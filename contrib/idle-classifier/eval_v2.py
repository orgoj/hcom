#!/usr/bin/env python3
"""Binary attention eval of a draft contract (needs_user yes/no + P(yes)) on cases.jsonl.

Usage: python3 -I eval_v2.py CONTRACT MODEL [MAX_CASES]   (verify with MAX_CASES=1 first)
Gold attention: manual `label` when set, else the Haiku label; attention = needs_user or blocked.
Writes <contract stem>_<model>-runs.jsonl to the private data directory and
results/<contract stem>_<model>-summary.json here.
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
CONTRACT_PATH = pathlib.Path(sys.argv[1])
CONTRACT = json.loads(CONTRACT_PATH.read_text())
MODEL = sys.argv[2]
# One safe filename component; the digest keeps distinct tags (a/b:c, a_b:c) apart.
MODEL_STEM = re.sub(r"[^\w.-]", "_", MODEL)
if MODEL_STEM != MODEL:
    MODEL_STEM += "-" + hashlib.sha1(MODEL.encode()).hexdigest()[:8]
LIMIT = int(sys.argv[3]) if len(sys.argv) > 3 else None
BASE = "http://127.0.0.1:11434"
FIELD = CONTRACT["decision_field"]
VALUE_START = re.compile(r'"' + FIELD + r'"\s*:\s*"$')
ATTENTION = {"needs_user", "blocked"}
THRESHOLDS = [0.01, 0.02, 0.05, 0.1, 0.2, 0.3, 0.5]


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


def p_yes(logprobs):
    """P(yes) from the alternatives of the first token of the decision value."""
    acc = ""
    for tok in logprobs or []:
        if VALUE_START.search(acc):
            alts = tok.get("top_logprobs") or [{"token": tok["token"], "logprob": tok["logprob"]}]
            return sum(math.exp(a["logprob"]) for a in alts if a["token"].strip().lower().startswith("y"))
        acc += tok["token"]
    return None


def classify(text):
    req = json.loads(json.dumps(CONTRACT["request"]))
    req["model"] = MODEL
    req["messages"][1]["content"] = CONTRACT["user_message_template"].format(
        message=text[-CONTRACT["input_max_chars"]:])
    t0 = time.perf_counter()
    try:
        resp = api("/api/chat", req)
    except (urllib.error.URLError, TimeoutError) as e:
        return {"answer": None, "state": None, "p_yes": None, "wall_s": time.perf_counter() - t0, "raw": str(e)}
    content = resp["message"]["content"]
    try:
        parsed = json.loads(content)
    except json.JSONDecodeError:
        parsed = {}
    return {"answer": parsed.get(FIELD), "state": parsed.get("state"), "p_yes": p_yes(resp.get("logprobs")),
            "wall_s": time.perf_counter() - t0, "raw": content}


def main():
    cases = [json.loads(line) for line in (OUT / "cases.jsonl").open(encoding="utf-8")][:LIMIT]
    haiku = {r["id"]: r["haiku"] for r in map(json.loads, (OUT / "haiku_labels.jsonl").open(encoding="utf-8"))}
    runs = []
    for c in cases:
        first, second = classify(c["text"]), classify(c["text"])
        gold = c.get("label") or haiku.get(c["id"])
        runs.append({"id": c["id"], "lang": c["lang"], "gold": gold, "gold_att": gold in ATTENTION,
                     "answer": first["answer"], "state": first["state"], "p_yes": first["p_yes"],
                     "wall_s": first["wall_s"], "deterministic": first["raw"] == second["raw"],
                     "raw": first["raw"]})
    stem = f"{CONTRACT_PATH.stem}_{MODEL_STEM}"
    with open(OUT / f"{stem}-runs{'-probe' if LIMIT else ''}.jsonl", "w", encoding="utf-8") as f:
        f.writelines(json.dumps(r) + "\n" for r in runs)

    att = [r for r in runs if r["gold_att"]]
    other = [r for r in runs if not r["gold_att"]]
    hl = lambda r, t: r["p_yes"] is None or r["p_yes"] >= t  # no probability: highlight, never hide
    sweep = [{"threshold": t,
              "missed": sum(not hl(r, t) for r in att), "false_highlight": sum(hl(r, t) for r in other)}
             for t in THRESHOLDS]
    by_answer = {"missed": sum(r["answer"] != "yes" for r in att), "false_highlight": sum(r["answer"] == "yes" for r in other)}
    max_safe = min((r["p_yes"] for r in att if r["p_yes"] is not None), default=None)
    walls = sorted(r["wall_s"] for r in runs)
    summary = {
        "contract": CONTRACT_PATH.name, "model": MODEL, "cases": len(runs),
        "gold_attention": len(att), "gold_other": len(other),
        "by_answer": by_answer,
        "lowest_p_yes_on_gold_attention": max_safe,
        "false_highlight_at_that_threshold": sum(r["p_yes"] is None or r["p_yes"] >= max_safe for r in other) if max_safe is not None else None,
        "threshold_sweep": sweep,
        "invalid": sum(r["answer"] not in ("yes", "no") for r in runs),
        "missing_p_yes": sum(r["p_yes"] is None for r in runs),
        "deterministic": all(r["deterministic"] for r in runs),
        "latency_s": {"p50": round(statistics.median(walls), 3), "p95": round(walls[int(0.95 * (len(walls) - 1))], 3),
                      "max": round(walls[-1], 3)},
        "missed_ids_by_answer": [r["id"] for r in att if r["answer"] != "yes"],
    }
    if LIMIT is None:  # a verification run must not replace full-set files
        (RESULTS / f"{stem}-summary.json").write_text(json.dumps(summary, indent=2) + "\n")
    print(json.dumps(summary, indent=2))


if __name__ == "__main__":
    main()

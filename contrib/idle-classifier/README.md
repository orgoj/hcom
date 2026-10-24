# Idle-agent classifier: evaluation

Experiment for a future hcom-gui column: when an agent goes idle (`active` → `listening`),
decide from its last message whether the user must act now. Other states (`blocked` on an
approval, `active`) are deterministic in hcom and are not classified. Nothing here is wired
into hcom or the gui yet.

Requirement: no idle agent that waits for the user may stay unmarked. A few false highlights
are acceptable.

## Data and privacy

- Message texts never enter this repository. `build_eval_set.py` writes them to
  `$HCOM_DIR/tmp/idle-classifier/` (default `~/.hcom/tmp/idle-classifier/`): `cases.jsonl`,
  labels, per-case runs and `review.jsonl`.
- `results/` holds metrics only (counts, rates, latencies, case ids).
- `label_haiku.py` sends case texts to Claude Haiku (cloud); the user approved that step.
  Local model calls go only to the local Ollama.

## Files

| File | Purpose |
|---|---|
| `build_eval_set.py [--hcom BIN] [--batch NAME [--older N]]` | Idle points from hcom transcripts (`hcom transcript <name> 1- --json`, `final_text`, else `action`); weak label from the next user turn (human / hcom / system) and cs/en keyword cues; balanced sample of up to 60 per weak label. `--batch` exports only texts no earlier export holds; by name it reaches only the newest incarnation of each instance. `--older N` instead reads every stopped incarnation's transcript file (`hcom transcript --path`), skips shared session databases (OpenCode, Kilo, Hermes) and keeps a seed-42 sample of N unseen idle points. |
| `label_haiku.py [--rules v1\|v2] [--source cases\|candidates] [--max-batches N]` | Labels from Haiku, batches of 10; per-call metrics and a run summary in `results/`. v2 = contract v2 rules with `must_act`. Resumes, never labels an id twice. |
| `contract_v1.json` | local_models' contract v1 (4 labels, `qwen3.5:4b`), copied for reproducibility. |
| `eval_local.py [MODEL [MAX_CASES]]` | v1 eval: accuracy, confusion, attention misses, threshold sweep; writes `review.jsonl`. |
| `contract_v2.json` | Draft prompt v2: primary binary question `needs_user` yes/no, explicit implicit-ask rules, user need wins, "if unsure: yes". Decoding as v1. |
| `eval_v2.py CONTRACT MODEL [MAX_CASES]` | Binary eval on P(yes) from the logprobs of the decision token. |
| `attention_data.py export LABELS \| check LABELS SCORES \| fresh BATCH \| train NAME [BATCH...]` (LABELS `v1` = 174 cases, `v2` = all candidates with Haiku v2 `must_act`) | Labels for local_models' `attention.py` (RobeCzech encoder + head); stratified 20 % holdout; held-out check; `fresh` turns a whole `--batch` export into an untouched held-out set; `train` joins `candidates.jsonl` and fully labelled batches into one training file and refuses frozen held-out batches. |

Every new external call is verified on exactly one call before a full run. Local evals
(`MAX_CASES=1`) write `*-probe.jsonl` and never replace full-set results. For Haiku the
verification is `--max-batches 1`, the first batch of the run itself, so no case is sent twice.
File names carry the model as one safe component; a tag that needs escaping gets a digest of
the raw name (`qwen3.5:4b` → `qwen3.5_4b-c57d0c95`).

## Results (174 real cases, 2026-10-24)

Dataset: 622 idle points from Claude, Codex, Antigravity and Hermes; the sample has
done 60, needs_user 60, waiting_agent 43, blocked 11 by weak label. Gold for now = Haiku
labels (94 attention = `needs_user` ∪ `blocked`, 80 other); the user's manual pass is
pending (see Reviewed corrections for the agent-reviewed fixes so far).

| `qwen3.5:4b` | contract v1 (4 labels) | prompt v2 (yes/no) |
|---|---|---|
| Missed attention | 6/94 | 0/94 |
| False highlight | 5/80 | 10/80 |
| Invalid / deterministic | 0 / yes | 0 / yes |
| Warm latency p50 / p95 | 0.76 / 1.48 s | 0.99 / 1.77 s |

- v1 misses were all Czech implicit asks: an earlier question restated as still open, a
  decision left to the user, proposed changes not applied, an instruction for the user, an
  "open items" list of questions. One miss had confidence 0.91, so no v1 threshold fixes it:
  0 misses would need 52/80 false highlights.
- v2: the lowest P(yes) on a gold attention case is 0.648; highlighting at P(yes) ≥ 0.5 gives
  0 misses and 10 false highlights.
- Haiku labeller: 18 calls (≤ 10 cases each), p50 5.1 s per call, about 0.017 USD for 174
  cases, 168/174 identical labels across two runs. Regex weak labels agree with Haiku on 0.707.

### Contract v2: RobeCzech encoder + trained head

The user's target is a Jev-class decision model (milliseconds, < 1 GB VRAM); local_models
withdrew the chat models. Head trained on 139 cases (75 must_act, 122 Czech), 35 held out:

| | Result |
|---|---|
| CV AUC | 0.879 (cs 0.860, en 0.188 on 17 English cases) |
| CV zero misses | threshold 0.0014, 50/64 false highlights |
| CV 2 % / 5 % miss budget | 1 missed, 30 FH / 3 missed, 26 FH |
| Held-out at default threshold | 1/19 missed, 11/16 false highlights |
| Latency | p50 19 ms per message |

Not usable yet: zero misses costs about 78 % false highlights
(`results/attention-head-cv.json`, `results/attention-holdout-summary.json`).

More labels: Haiku labelled all 622 candidates once with the contract v2 rules
(`--rules v2 --source candidates`): 257 must_act; 65 calls, 0.118 USD, p50 5.8 s per call,
about 0.6 s per case (`results/haiku_v2_candidates-*`). On the 174 shared cases it agrees with
the v1 attention labels except 7 cases v2 now marks must_act. Head on 497, 125 held out:

| | Result |
|---|---|
| CV AUC | 0.886 (cs 0.857, en 0.735) |
| CV zero misses | threshold 0.0002, 247/292 false highlights |
| CV 2 % / 5 % miss budget | 4 missed, 158 FH / 10 missed, 120 FH |
| Held-out at default threshold | 0/52 missed, 60/73 false highlights |

Fresh export `2026-10-24b`: 141 idle points no earlier export held (cs 125), Haiku v2: 65
must_act; 15 calls, 0.028 USD (`results/dataset-2026-10-24b-summary.json`,
`results/haiku_v2_candidates-2026-10-24b-*`). Kept whole as an untouched held-out set.

AUC stays near 0.88 from 139 to 497 cases: the frozen encoder with a linear head, not the
label count, limits it (`results/attention-head-v2-cv.json`,
`results/attention-holdout-v2-summary.json`).

### Reviewed corrections

`$HCOM_DIR/tmp/idle-classifier/manual_must_act.jsonl` holds reviewed must_act corrections,
one JSON object per line, e.g.
`{"id": "fd4bd73f2f3c", "must_act": false, "reviewer": "hcom_dev", "note": "finished report"}`.
`id` (unique) and boolean `must_act` are required; `reviewer` and `note` are optional
provenance and are not read. They apply to v2 labels
only (v1 results stay reproducible). Precedence: a `label` on the candidate or in
`cases.jsonl` > `manual_must_act.jsonl` > Haiku. A frozen fresh set never changes, so a
correction of a fresh id makes `fresh` refuse instead of rewriting it.

Current content: 21 corrections by hcom_dev (agent review, not the user's manual pass) from
the 37 must_act calibration cases that the 3622-label v3.1 model scored below 0.1. 21 were
noise (finished reports, own next-step plans, waiting for another agent), 9 genuine asks,
7 ambiguous kept must_act. Training file with them: `labels-train-2026-10-24-r1.jsonl`
(3622 rows, 1605 must_act). Haiku's must_act is consistent with its own state, so the noise
is Haiku calling plain reports needs_user; more is likely among higher-scoring cases.

## Caveats

- Gold is Haiku labelled with the v1 rules, so it shares their blind spots. Final gold needs
  the user's manual pass, including a sample of cases where all labellers agree on "no";
  only the 21 agent-reviewed corrections above exist so far.
- Prompt v2 was written after seeing the v1 misses on this same set; its 0 misses may be
  optimistic until measured on fresh cases.
- Only 11 `blocked` cases by weak label (5 by Haiku).
- The target is a small decision model with millisecond responses and minimal resources;
  local_models owns that choice. `qwen3.5:4b` here is a baseline, not the target model.

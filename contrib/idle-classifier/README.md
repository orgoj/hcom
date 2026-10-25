# Idle-agent classifier: evaluation

Classifier for the hcom-gui Idle column: when an agent goes idle (`active` → `listening`),
decide from its last message whether the user must act now. Other states (`blocked` on an
approval, `active`) are deterministic in hcom and are not classified. Nothing here is wired
into the hcom binary; `idle_check.py` is the contract v4 consumer, standalone and run by
hcom-gui for its Idle column (`../README.md`).

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
| `final_inputs.py [--hcom BIN] OUT EXPORT...` | Inputs for the 3-class task from earlier exports: per idle point `final` (`true` when hcom recorded active → listening before the next exchange, `false` when the next input interrupted a running turn, `"unknown"` for instances hcom did not know then) and the input text (final_text; a tool-free reply; a reply ending on a tool call plus its last command when that is an `hcom send`), last 3000 characters. OUT is written once; counts in `results/final-inputs-OUT-summary.json`. |
| `idle_check.py [--json] [--hcom BIN] [--python PY] [--attention FILE] [--head DIR] [NAME...]` | Which listening agents need a user decision, per local_models' contract v4 (`~/work/local-models/idle-classifier/HANDOVER.md`): each listening agent's (only the NAMEs, when given) last exchange (`hcom transcript NAME --last 1 --json --detailed`) becomes the input as `final_inputs.py` builds it, all inputs go to one `attention.py score HEAD` call (default head `$HCOM_DIR/tmp/idle-classifier/head-3class-xlmr-r5-e21`, its `default_threshold`; defaults for PY and FILE in `~/work/local-models/`). Prints name, class (`needs_user` / `waiting` / `done`) and p(needs_user), needs_user first; `--json` rows `{name, tool, class, p_needs_user, p, input_kind, status_age_seconds, reason}`. Unscored rows have `class: null` and a reason (`no_exchange`, `empty_text`, `transcript_failed`, `transcript_invalid`, or `stale` when the agent left listening or its last exchange changed during the run). Never prints message text; failures (exit 1) name a category, never command output. Tests: `python3 -I test_idle_check.py`. |
| `label_haiku.py [--rules v1\|v2\|v3] [--model haiku\|sonnet] [--source STEM] [--ids FILE] [--max-batches N]` | Blind labels (message text + rules only) from Haiku or Sonnet, batches of 10; per-call metrics and a run summary in `results/`. v2 = contract v2 rules with `must_act`; v3 = contract v3 `class` with `must_act` = needs_user. `--source` is any private JSONL with id and text, `--ids` limits the run to listed ids. Resumes, never labels an id twice. |
| `contract_v3.json` | 3-class labelling rules for final messages: `needs_user` / `waiting` (another agent, script, review; must not be stopped) / `done` (can be stopped), plus `not_final` for intermediate progress notes. needs_user wins, then waiting before done. |
| `compare_labels.py BASE SECOND OUT [--ids FILE]` | Agreement of a labels file with a second blind labelling (e.g. Sonnet), and a corrected copy OUT (new name, written once) that takes SECOND's label for the compared ids; summary with disagreeing ids in `results/compare-OUT.json`. |
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

Canonical gold now lives in local_models' per-case records
`$HCOM_DIR/tmp/idle-classifier/cases/<id>.json` with the user's precedence
user > Sonnet > hcom_dev agent review (`manual_must_act.jsonl`) > Haiku. The user settled the
4 conflicts between agent review and Sonnet: 56225cb19b6a, 425b463dc67e, bdef3bf39810 are
must_act (Sonnet right), 960ff6c1d551 is ambiguous. The label files built by
`attention_data.py` predate this and do not apply Sonnet.

Current content: 21 corrections by hcom_dev (agent review, not the user's manual pass) from
the 37 must_act calibration cases that the 3622-label v3.1 model scored below 0.1. 21 were
noise (finished reports, own next-step plans, waiting for another agent), 9 genuine asks,
7 ambiguous kept must_act. Training file with them: `labels-train-2026-10-24-r1.jsonl`
(3622 rows, 1605 must_act). Haiku's must_act is consistent with its own state, so the noise
is Haiku calling plain reports needs_user; more is likely among higher-scoring cases.

### Sonnet check of disputed cases

Blind Sonnet (`claude-sonnet-5-5`, contract v2 rules, text only) on cases where the
v3.1-3622r1 model and the Haiku label disagree (must_act scored < 0.1, or not must_act scored
≥ 0.5):

| Set | Compared | Agree | Haiku→Sonnet changes | Corrected file |
|---|---|---|---|---|
| Training calibration split | 70 | 50 | 12 true→false, 8 false→true | `labels-train-2026-10-24-r2-sonnet.jsonl` (1601 must_act) |
| Fresh held-out, disputed | 19 | 16 | 1 true→false, 2 false→true | `labels-fresh-2026-10-24b-sonnet-disputed.jsonl` (66 must_act) |

Frozen XLM-R large head (out-of-fold over 3622) vs the r2 gold: 322 disputed training cases,
Sonnet agrees with gold on 265; 14 true→false, 43 false→true; 33 calls, 1.019 USD. Corrected
copy `labels-train-2026-10-24-r3-sonnet.jsonl` (1630 must_act). Sonnet contradicts 4 of the
10 agent-review corrections it saw (`compare-labels-train-2026-10-24-r3-sonnet.json`).
User rule since then: every model-vs-gold disagreement gets a blind Sonnet check, and
Sonnet labels rank above Haiku; local_models keeps one record per case in `cases/<id>.json`.

A Sonnet run on all 141 fresh cases happened before the scope was narrowed and is kept
separate: 130/141 agree, 9 true→false, 2 false→true (`labels-fresh-2026-10-24b-sonnet.jsonl`).
Cost: training 0.209 USD, fresh 0.436 USD. Originals stay untouched.

## Three-class cycle (final messages)

The user redefined the task: classify only the final message of an agent idle in listen as
needs_user, waiting or done, and found the earlier inputs wrong. Results above (binary,
old inputs) are superseded; the cycle restarts in this order: re-export, Haiku v3 on all
in-scope cases, existing models raw on the new inputs, blind Sonnet v3 on every disagreement
(precedence user > Sonnet > Haiku), only then training and held-out evaluation.

- Re-export `inputs-3class` (hcom 0.7.30-orgoj.17, from `candidates`, `candidates-2026-10-24b`,
  `candidates-2026-10-24-older`): 3745 rows, final 3084 / not final 325 / unknown 336, 18
  cases no longer in transcripts dropped (`results/final-inputs-inputs-3class-summary.json`).
- Haiku v3 on the 3420 final or unknown ids: needs_user 1443, waiting 771, done 1067,
  not_final 139; 343 calls, 0.655 USD (`results/haiku_v3_inputs-3class-8892593d-*`).
- `inputs-3class` is wrong for Codex: hcom ignored Codex code-mode tool calls
  (`custom_tool_call` `exec`), so those turns had no final_text and their input was the whole
  reply. Fixed in hcom 0.7.30-orgoj.18. `inputs-3class-r2` is a defective run: bare-name
  references of instances that had handed off since resolved to the new session (116 rows
  missing). `inputs-3class-r3` resolves them to the earlier transcript by text: 3748 rows,
  final 3092 / not final 313 / unknown 343, 15 missing; 607 texts differ from
  `inputs-3class` (568 Codex), final flags are unchanged. Haiku v3 relabels only the 548
  in-scope ids whose input changed or is new (`haiku_v3_inputs-3class-r3-*`): needs_user 101,
  waiting 151, done 276, not_final 20; 56 calls, 0.072 USD. Other ids keep their
  `inputs-3class` Haiku label (identical input).
- `inputs-3class-r5` is current: r3 minus 4 rows whose source exchange is ambiguous; every row
  is verified by the export id hash (3117) or a unique text match (627). `inputs-3class-r4` is
  a defective run (the same file reached by name and by path counted as 589 ambiguities).
- Blind Sonnet v3 on the 441 model disagreements whose input did not change
  (`results/sonnet_v3_inputs-3class-r3-*`, 1.594 USD): agrees with Haiku on 356; by Haiku
  class done 163/215, waiting 175/198, needs_user 18/28; Sonnet adds 47 needs_user
  (done→needs_user 43, waiting→needs_user 14, needs_user→other 10).
- Then on the 59 model disagreements among the 548 relabelled ids (same Sonnet file, 500 ids,
  1.728 USD in total): agrees with Haiku on 36; by Haiku class done 14/22, waiting 9/11,
  needs_user 13/26 (needs_user→waiting 10). Step 4 overall: 392/500 agree; Sonnet adds 63
  needs_user and removes 23. The summary file describes the last `--ids` run only.

## Caveats

- A turn that starts with a tool call and then has text gets an empty `final_text` from
  hcom (`final_segment` treats start 0 as no boundary); `final_inputs.py` then uses the whole
  reply, which is the correct text, and records it as `tool_ended_other`.

- Gold is Haiku labelled with the v1 rules, so it shares their blind spots. Final gold needs
  the user's manual pass, including a sample of cases where all labellers agree on "no";
  only the 21 agent-reviewed corrections above exist so far.
- Prompt v2 was written after seeing the v1 misses on this same set; its 0 misses may be
  optimistic until measured on fresh cases.
- Only 11 `blocked` cases by weak label (5 by Haiku).
- The target is a small decision model with millisecond responses and minimal resources;
  local_models owns that choice. `qwen3.5:4b` here is a baseline, not the target model.

---
id: TASK-121
title: agy delivery stalls after a phase-1 render timeout
status: Done
assignee:
  - '@hcom_dev__fix'
created_date: '2026-11-19T09:45:00Z'
updated_date: '2026-11-19T10:05:00Z'
labels: []
dependencies: []
priority: high
type: bug
ordinal: 232000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Observed 2026-11-19: knowledge -> agent_architect_killbottleneck_228 (agy 1.5.12), message #396320 cut to 1500 chars (AGY_PROMPT_MAX_CHARS). agy rendered ~130 chars/s, so after 10 s (PHASE1_TIMEOUT) only ~1330 chars showed; delivery.phase1_timeout -> State::Pending. The prompt then holds our own typed text, agy reports ready=false, evaluate_gate returns not_ready forever (500+ attempts) and Enter is never sent. Fix (user decision, variant A): in Pending, after a phase-1 timeout, when the prompt holds exactly our last typed agy text and the agent is idle, return to the render phase (which re-checks ownership, approval and typing) and submit it. Variant B (extend the timeout while the render grows) dropped: A covers it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 After a phase-1 timeout, a prompt that holds exactly the typed agy text is submitted once the agent is idle, without retyping
- [x] #2 A prompt with other or additional text is never submitted by this path
- [x] #3 Unit test covers the decision
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
After a phase-1 timeout with a typed agy prompt (prompt_ack set), Pending re-enters WaitTextRender once the prompt holds exactly our text and the agent is idle (resume_late_render); the render phase re-checks ownership, approval and typing before Enter. Variant B dropped as redundant. Verified: unit test late_render_resumes_only_our_exact_prompt_once_idle, cargo test --bin hcom, cli_smoke antigravity, fmt, clippy; review OK by agent_review.
<!-- SECTION:FINAL_SUMMARY:END -->

---
id: TASK-114
title: 'AGY: mid-turn delivery lost (ephemeralMessage shown to one model call only)'
status: Done
assignee: []
created_date: '2026-11-16T10:54:00Z'
updated_date: '2026-11-16T13:29:00Z'
labels:
  - bug
dependencies: []
priority: high
ordinal: 225000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
knowledge (agy) asked @local_models at 10:45:40; reply #375727 arrived 10:46:03 and hcom delivered it at 10:46:04 via PreInvocation injectSteps[].ephemeralMessage (status deliver:local_models, AGY log step 49). The very next model call (step 50) read it in its thinking, but ephemeral steps are not persisted in the trajectory (src/hooks/gemini.rs:275), so the following ~15 calls no longer saw it and the final answer (10:47:06) said 'STATUS: WAITING for @local_models'. hcom marked the message read, so it is never redelivered. Not covered by upstream (13 commits behind, v0.7.31) or any fork remote (checked 2026-11-16).
<!-- SECTION:DESCRIPTION:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Options for the user to choose: A) keep re-injecting messages delivered during the current agy turn on every PreInvocation until Stop (small context cost, only within one turn); B) agy: never deliver mid-turn via ephemeral, only at turn boundary through PTY so the message becomes a persisted user step (delivery delay); C) at agy Stop, if messages were delivered during the turn, re-deliver them as a new PTY turn. Recommendation: C (persisted, no per-call bloat, reuses existing PTY delivery).
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
User chose B with bodies in the prompt. Implemented: agy PTY types all pending bodies as one prompt-safe line (no @, no word starting with >, max 1500 chars, longer cut with an 'hcom events --sql id range' pointer, request note), PTY commits the ack once agy clears the prompt; gemini-beforeagent delivers no bodies while a pty endpoint exists (non-PTY agy unchanged); agy screen parser joins wrapped rows (two-space indent); prompt ownership also matches whitespace-insensitively (wrap inside a long word). Probe on agy 1.5.9: input wraps on 2-space-indented rows; render ~330 chars/s, slower as it grows. Live check (debug build): body is USER_INPUT step, model kept the content across 6 tool steps. Diff /home/michael/projects/hcom/tmp/task-114-diff.patch sent to agent_review.

Review (agent_review) approved after 2 rounds: ack only after hcom's own Enter cleared the prompt and agy's turn-start hook set active:prompt (cancel keeps messages pending); text verbatim except newlines/tabs; messages that do not fit stay pending; a single message over 1500 chars is a documented delivery exception (cut + hcom events --sql id=N pointer). Live-checked on agy 1.5.9 with the debug build.
<!-- SECTION:NOTES:END -->

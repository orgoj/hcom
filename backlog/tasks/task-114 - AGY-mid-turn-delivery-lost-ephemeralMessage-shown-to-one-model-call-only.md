---
id: TASK-114
title: 'AGY: mid-turn delivery lost (ephemeralMessage shown to one model call only)'
status: To Do
assignee: []
created_date: '2026-11-16T10:54:00Z'
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

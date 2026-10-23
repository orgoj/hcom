---
id: TASK-42.1
title: Mark handoff relaunches in life events
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-10-23 14:07'
updated_date: '2026-10-23 14:42'
labels: []
dependencies: []
parent_task_id: TASK-42
priority: medium
type: feature
ordinal: 43000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: hcom-gui statistics (TASK-42) need a handoff count, but `hcom handoff` relaunches through `hcom agent <name> --as <name> --restart --clean`, so its stop/start life events are indistinguishable from a plain restart.

Design (approved by agent_review): `hcom handoff` logs one life event `{"action":"handoff","by":<name>,"cli":<--cli or null>}` for the instance after the synchronous --dry-run validation passed and the detached relaunch was spawned. It counts a validated relaunch attempt: a successor that fails afterwards is still counted; its failure is in ~/.hcom/logs/handoff-<name>.log and shows as a missing ready. Nothing passes through the environment, so nothing can leak to the successor or its children.
Out of scope: counting/displaying (TASK-42); backfilling past handoffs.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A non-dry-run hcom handoff that passes validation logs exactly one life event with action handoff for the handing-off instance, selectable with hcom events --action handoff
- [x] #2 hcom handoff --dry-run, a failed validation, a plain hcom agent <name> --restart and a hcom-gui Restart log no handoff event
- [x] #3 The handoff action is listed with its attempt semantics in the life_action help (src/commands/help.rs) and hcom events --action help, and in README/docs where life actions are described
- [x] #4 A test covers the logged event and the absence on dry-run/failed validation
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
hcom handoff logs life action 'handoff' (by, cli) after validation and the detached relaunch spawn; 'handoff' added to hcom events --action values and life_action help, handoff help and README. Verified with cli_smoke handoff tests (dry-run, failed validation and plain --restart log none; real handoff logs one); reviewed by agent_review.
<!-- SECTION:FINAL_SUMMARY:END -->

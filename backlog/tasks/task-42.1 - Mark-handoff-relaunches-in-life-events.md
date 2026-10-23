---
id: TASK-42.1
title: Mark handoff relaunches in life events
status: To Do
assignee: []
created_date: '2026-10-23 14:07'
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

In scope: the binary records that a stop/launch was caused by `hcom handoff`, in the existing life events (no new event type), so `hcom events --type life --sql ...` can filter it; docs of the event field.
Out of scope: counting or displaying (TASK-42); backfilling past handoffs.
Risk: the marker is invocation-local and must not leak into the successor's environment or its child launches (AGENTS.md per-CLI env rule).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 After a successful hcom handoff, the life events of that handoff (stop of the old session and launch/ready of the successor) carry a handoff marker that an hcom events --sql filter selects
- [ ] #2 A plain hcom agent <name> --restart and a hcom-gui Restart produce life events without the handoff marker
- [ ] #3 Agents launched by the successor do not inherit the marker (their life events carry none)
- [ ] #4 A cli_smoke or unit test covers both the marked handoff and the unmarked restart; the event field is documented in README/docs and hcom events help where life event fields are described
<!-- AC:END -->

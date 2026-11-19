---
id: TASK-130
title: 'hcom-dashboard: Idle classifier off by default, opt-in via dashboard.json'
status: To Do
assignee: []
created_date: '2026-11-19T15:57:00Z'
updated_date: '2026-11-19T15:57:00Z'
labels: []
dependencies: []
priority: high
type: enhancement
ordinal: 241000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
User 2026-11-19: the dashboard Idle column (contrib/hcom-dashboard/src/idle.rs, added in TASK-77.12) runs contrib/idle-classifier/idle_check.py per agent nonstop (10-12+ s per run, each calling hcom list --json; app.rs starts the next run as soon as one finishes while the Idle column is shown). It is unreliable and costly; the agent-written status already shows the state and works. Keep the code (may be used later), but disable it by default behind a key in ~/.hcom/dashboard.json. Related: TASK-128 (dashboard CPU cost, pure-Rust decision).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 With no key in dashboard.json the dashboard never spawns idle_check.py or any process for the Idle column
- [ ] #2 Setting the key to true in dashboard.json restores the current Idle column behavior unchanged
- [ ] #3 With the feature off the Idle column is not offered in the UI
- [ ] #4 Tests cover the default-off and opt-in cases
- [ ] #5 The key is documented in contrib/hcom-dashboard/README.md next to the other dashboard.json keys
<!-- AC:END -->

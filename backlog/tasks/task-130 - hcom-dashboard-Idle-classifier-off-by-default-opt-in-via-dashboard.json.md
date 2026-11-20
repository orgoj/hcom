---
id: TASK-130
title: 'hcom-dashboard: Idle classifier off by default, opt-in via dashboard.json'
status: Done
assignee: []
created_date: '2026-11-19T15:57:00Z'
updated_date: '2026-11-20T17:23:00Z'
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
- [x] #1 With no key in dashboard.json the dashboard never spawns idle_check.py or any process for the Idle column
- [x] #2 Setting the key to true in dashboard.json restores the current Idle column behavior unchanged
- [x] #3 With the feature off the Idle column is not offered in the UI
- [x] #4 Tests cover the default-off and opt-in cases
- [x] #5 The key is documented in contrib/hcom-dashboard/README.md next to the other dashboard.json keys
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-11-20: key idle_classifier (bool, default false) in DashboardConfig. App::table_columns() filters 'idle' from shown columns (also a saved dashboard-state.json choice), column_choices() from the Columns dialog, report view from its fields; idle_column_shown() derives from table_columns(), so update_idle/poll_idle never queue or start IdleJob when off. Code in idle.rs unchanged. Tests: test_idle_classifier_off_by_default_hides_column_and_queues_nothing, opt-in in test_idle_column_queues_listening_periods_only_while_shown.
<!-- SECTION:NOTES:END -->

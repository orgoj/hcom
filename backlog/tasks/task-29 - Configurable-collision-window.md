---
id: TASK-29
title: Configurable collision window
status: To Do
assignee: []
created_date: '2026-10-16 05:53'
labels: []
dependencies: []
type: feature
ordinal: 28000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
File-edit collision detection uses a hardcoded 30 s window (collision_window_sql in src/core/filters.rs); the user finds it too short. Add a config key collision_window (seconds, default 30, env HCOM_COLLISION_WINDOW) used by 'hcom events --collision', 'hcom events sub --collision' and the auto_subscribe collision preset. Collision subscriptions store their SQL in the database; existing ones must pick up the configured window (upgrade_collision_sql already rewrites stored collision SQL). A longer window raises false positives (sequential handoff edits of one file).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 'hcom config collision_window <seconds>' changes the window for events --collision, events sub --collision and new auto-subscribed agents; default stays 30
- [ ] #2 Collision subscriptions stored before the change use the configured window after it (no mix of old and new windows)
- [ ] #3 Invalid values (non-integer, zero, negative) are rejected with a clear error
- [ ] #4 Help texts that state '30s' reflect the configured window or the key; 'hcom config collision_window --info', README.md, skills/hcom-agent-messaging/ and tests (unit for the SQL window, cli_smoke for a collision outside 30 s but inside the configured window) cover it
<!-- AC:END -->

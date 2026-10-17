---
id: TASK-29
title: Configurable collision window
status: To Do
assignee: []
created_date: '2026-10-16 05:53'
updated_date: '2026-10-17 06:59'
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

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Deferred (user: no change to stored subscription data). Analysis: collision_window_sql (src/core/filters.rs) hardcodes 30 s in three literals ('-30 seconds', '+31 seconds', '< 30'). Users: build_sql_from_flags (events --collision, events sub --collision, auto_subscribe preset), collision_self_relevance_sql and find_collision_partner (src/db/subscriptions.rs), upgrade_collision_sql. Subscriptions store the full SQL in kv events_sub:*, and the sub id hashes that SQL, so a configurable window means (1) rewriting stored SQL to the current window (e.g. extend the upgrade_collision_sql hook in process_logged_event), (2) hashing the id with a normalized window to avoid duplicate auto-subscriptions after a change, (3) reading config in every hook run with collision subs (or a per-process cache that long-lived processes keep stale). A variant without touching stored data would need the collision window resolved at evaluation time instead of baked into stored SQL - a larger change to the subscription design.
<!-- SECTION:NOTES:END -->

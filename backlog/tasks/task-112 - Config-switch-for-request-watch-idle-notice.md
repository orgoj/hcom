---
id: TASK-112
title: Config switch for request-watch idle notice
status: Done
assignee: []
created_date: '2026-11-13T21:40:00Z'
updated_date: '2026-11-13T21:49:00Z'
labels: []
dependencies: []
ordinal: 223000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Request-watch (send --intent request) notifies the sender when the recipient goes idle without replying. Agents that wait on another agent or a running program get it as noise. Add config key request_idle_notify (default true = current behavior); false keeps only the 'stopped without responding' notice. Disable it on this machine.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 hcom config request_idle_notify false: new request watches fire only on stopped
- [x] #2 Default unchanged
- [x] #3 Help, README, skill updated; set false on this machine
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Config key request_idle_notify (preferences, default true). 2. create_request_watches(idle_notify): false -> SQL only life stopped. 3. send.rs passes config. 4. Help/README/skill. 5. Review agent_review, commit, release build, then hcom config request_idle_notify false on this machine.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented, gates green; diff tmp/task-112.diff with agent_review. Existing watches created before the switch still carry the idle condition (short-lived).

Review agent_review: launcher exported the key into agent env, pinning running agents; fixed by omitting it from to_env_dict. Set false on this machine.
<!-- SECTION:NOTES:END -->

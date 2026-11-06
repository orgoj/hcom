---
id: TASK-94
title: Life created.by names the launcher's launcher
status: To Do
assignee: []
created_date: '2026-11-06T22:33:00Z'
labels: []
dependencies: []
priority: medium
type: bug
ordinal: 142000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Life event created.by is taken from the launcher process's own HCOM_LAUNCHED_BY, so it names the launcher's launcher instead of the launching agent (observed: wdt_traffic shows cca_manager instead of wdt_main). Found while building the hcom-dashboard spawn tree (TASK-77.38).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 created.by of a newly launched agent names the agent that ran the launch
- [ ] #2 Regression test in cli_smoke covers a nested launch (A launches B, B launches C: C's created.by is B)
<!-- AC:END -->

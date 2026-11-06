---
id: TASK-94
title: Life created.by names the launcher's launcher
status: Done
assignee:
  - '@hcom_dev__idea'
created_date: '2026-11-06T22:33:00Z'
updated_date: '2026-11-06T22:44:00Z'
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
- [x] #1 created.by of a newly launched agent names the agent that ran the launch
- [x] #2 Regression test in cli_smoke covers a nested launch (A launches B, B launches C: C's created.by is B)
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Launcher pre-creates the child row in its own process; pass launcher_name into initialize_instance_with_launcher so created.by/is_hcom_launched come from the launch, not the launcher's inherited env.
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The launcher process pre-creates the child row, so created.by read the launcher's own inherited HCOM_LAUNCHED_BY (its parent). initialize_launch_instance now passes launcher_name to initialize_instance_with_launcher (upstream initializer kept as a wrapper), which also sets is_hcom_launched for user-shell launches. Verified by cli_smoke nested_launch_created_event_names_the_launching_agent (fails on unfixed source), cargo test --bin hcom, fmt, clippy; reviewed OK by agent_review.
<!-- SECTION:FINAL_SUMMARY:END -->

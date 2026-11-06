---
id: TASK-77.47
title: >-
  hcom-dashboard: session boundaries limited by the 100000 life-event cap
  (TASK-77.44 review)
status: Done
assignee: []
created_date: '2026-11-06T20:54:00Z'
labels:
  - review:rejected
dependencies: []
parent_task_id: TASK-77
priority: low
type: bug
ordinal: 140000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Review of TASK-77.44 (agent_review): ReloadStats loads life events with --last 100000, while initiators use the full retained message history, so created events older than the cap would merge sessions.
<!-- SECTION:DESCRIPTION:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Rejected: the live database holds 12879 life events (3250 created), about 8x below the cap; the same cap already bounds the existing stats (starts, runtime, launcher). Internal tool: no extra uncapped query for a theoretical edge case. Revisit only if life events approach the cap.
<!-- SECTION:FINAL_SUMMARY:END -->

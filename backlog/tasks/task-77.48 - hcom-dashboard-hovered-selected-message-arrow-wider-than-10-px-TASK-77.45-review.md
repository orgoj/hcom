---
id: TASK-77.48
title: >-
  hcom-dashboard: hovered/selected message arrow wider than 10 px (TASK-77.45
  review)
status: Done
assignee: []
created_date: '2026-11-06T21:03:00Z'
labels:
  - review:rejected
dependencies: []
parent_task_id: TASK-77
priority: low
type: bug
ordinal: 141000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Review of TASK-77.45: the largest flow is 10 px, hover adds 1 px and selection 2 px, so the drawn stroke reaches 11-12 px.
<!-- SECTION:DESCRIPTION:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Rejected: the +1/+2 px is the pre-existing hover/selection highlight, not flow width; TASK-77.45 AC states it explicitly.
<!-- SECTION:FINAL_SUMMARY:END -->

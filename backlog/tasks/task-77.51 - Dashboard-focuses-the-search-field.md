---
id: TASK-77.51
title: 'Dashboard: / focuses the search field'
status: Done
assignee: []
created_date: '2026-11-06T23:40:00Z'
updated_date: '2026-11-06T23:42:00Z'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: low
type: bug
ordinal: 146000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The search field hint promises '(/ to focus)', but no shortcut exists.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Pressing / while no text field has the keyboard focuses the search field, without typing / into it
- [x] #2 / typed inside a text field stays ordinary input
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Typed '/' (text event, so any keyboard layout) focuses the search field when no text field has the keyboard; inside a field it stays input. Verified by test_task_77_51_slash_focuses_the_search_field (fails without the fix) and dashboard fmt/clippy/tests (17+188). Small change, no review.
<!-- SECTION:FINAL_SUMMARY:END -->

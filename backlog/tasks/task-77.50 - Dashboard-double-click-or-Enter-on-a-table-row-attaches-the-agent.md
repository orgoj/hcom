---
id: TASK-77.50
title: 'Dashboard: double-click or Enter on a table row attaches the agent'
status: Done
assignee:
  - '@hcom_dev__idea'
created_date: '2026-11-06T23:32:00Z'
updated_date: '2026-11-06T23:37:00Z'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: medium
type: bug
ordinal: 145000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Double-clicking a graph node attaches the agent (herdr focus), but the table row has no such gesture. User decision: like the node, a single click and Up/Down only select; a double-click on a row or Enter on the selected row attaches a running agent.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Double-clicking a table row of a running agent selects it and dispatches Attach, like a double-click on its graph node
- [x] #2 Enter with a selected row of a running agent dispatches Attach, except while a text field has keyboard focus
- [x] #3 Rows of agents that are not running dispatch no Attach
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Table rows (cells and the row band) attach a running agent on double-click, and Enter attaches the selected running row, like a node double-click. Enter is ignored while a dialog is open or a text field had focus at frame start (a single-line field drops focus on Enter before the table reads it). Verified by test_task_77_50_table_row_double_click_or_enter_attaches and dashboard fmt/clippy/tests (17+187); reviewed OK by agent_review.
<!-- SECTION:FINAL_SUMMARY:END -->

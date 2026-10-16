---
id: TASK-34
title: 'hcom-gui: Up/Down move through agents from any main-window widget'
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-10-16 15:59'
updated_date: '2026-10-16 17:28'
labels: []
dependencies: []
type: feature
ordinal: 33000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Up/Down move the agent selection even when the table does not have keyboard focus (e.g. after typing in the Filter field or clicking a button); the table then takes focus so Enter and button keys act on the selected row.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Up/Down pressed while focus is outside the table (e.g. the Filter field) move the selection by one row and give the table keyboard focus
- [x] #2 Up/Down inside the table keep the native Treeview behavior (one row per press)
- [x] #3 contrib/test-hcom-gui covers the cases above; contrib/README.md documents the keys
- [x] #4 With no selected row (in or outside the table), Down selects the first row and Up the last
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Up/Down bound on the main window and the table (_step_row): outside the table or with no selection they move/pick the row and focus the table; selected-row moves stay native. Verified with contrib/test-hcom-gui (68 OK) and agent_review.
<!-- SECTION:FINAL_SUMMARY:END -->

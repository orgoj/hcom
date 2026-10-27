---
id: TASK-55
title: 'hcom-gui: toolbar in two rows'
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-10-27 19:41'
updated_date: '2026-10-27 19:45'
labels: []
dependencies: []
type: feature
ordinal: 56000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The toolbar no longer fits on the screen. Split it into two rows: the first holds the command buttons (Start ... commands, Columns, Refresh), the second the rest: row count, Filter field and the filter checkboxes (Hide non-persistent, Hide disabled, Blocked only, Running only, Empty last).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The first toolbar row holds only buttons; the second row holds the row count, the Filter field and the checkboxes
- [x] #2 Keys, button states and the gui-state.json filters work as before
- [x] #3 contrib/test-hcom-gui passes and checks the two rows; contrib/README.md describes the layout if it mentions the toolbar placement
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Toolbar split into two rows: buttons (incl. Columns, Refresh) on the first, row count, Filter field and filter checkboxes on the second in the previous order. Variables, keys and gui-state.json unchanged. New test checks both rows; 134 tests OK. README updated. Reviewed by agent_review.
<!-- SECTION:FINAL_SUMMARY:END -->

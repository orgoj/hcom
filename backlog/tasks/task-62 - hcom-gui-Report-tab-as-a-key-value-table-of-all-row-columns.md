---
id: TASK-62
title: 'hcom-gui: Report tab as a key-value table of all row columns'
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-10-29 07:43'
updated_date: '2026-10-29 07:48'
labels: []
dependencies:
  - TASK-61
priority: medium
type: enhancement
ordinal: 63000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
User correction of TASK-61: the Report tab must show by default all columns of the selected row (built-in and status columns), not only status values, laid out as a key-value table. Values stay full and wrapped (TASK-61 decision); status values keep their age.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Without gui.json 'report' the Report tab lists every column of the selected row, built-in and status, one key-value line each (column title, value); the columns shown in the table come first in table order, then the rest in default column order
- [x] #2 Values are aligned in a value column and wrap with a hanging indent under it; status values show their full text and age since updated_at
- [x] #3 gui.json 'report' is a list of column keys like 'columns' (status:<name> included): the tab shows exactly those in that order; an unknown built-in key is logged, a status key no longer configured is dropped silently
- [x] #4 No selection: the tab is empty; contrib/test-hcom-gui covers each AC; contrib/README.md and the module docstring describe the new layout and 'report' keys
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Report tab of contrib/hcom-gui is a key-value table of the selected row: every column (built-in and status), shown table columns first, then the rest; title bold, value in an aligned column (tab stop + lmargin2, value newlines continue under it), status values full with age; keeps end/scroll position like the other tabs. gui.json report is now a list of column keys (unknown built-in keys logged, stale status keys dropped silently, also on reconcile). Tests: contrib/test-hcom-gui 159 OK. Docs: contrib/README.md, docstring. Reviewed by agent_review.
<!-- SECTION:FINAL_SUMMARY:END -->

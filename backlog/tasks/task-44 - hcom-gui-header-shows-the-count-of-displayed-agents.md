---
id: TASK-44
title: 'hcom-gui: header shows the count of displayed agents'
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-10-26 13:40'
updated_date: '2026-10-26 13:44'
labels: []
dependencies: []
type: feature
ordinal: 45000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
User request: the hcom-gui header shows how many agents are displayed. The count must respect all active filters, i.e. it equals the rows the user sees in the table.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The header shows the number of agent rows currently visible in the table
- [x] #2 The count updates when any filter changes or the agent list refreshes, and always matches the visible rows
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Window title shows 'hcom-gui — N agents', N = rows in the table after all filters, recomputed on every render (filter change, refresh). Verified with contrib/test-hcom-gui test_title_counts_the_shown_rows.
<!-- SECTION:FINAL_SUMMARY:END -->

---
id: TASK-44
title: 'hcom-gui: header shows the count of displayed agents'
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-10-26 13:40'
updated_date: '2026-10-26 13:57'
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
- [x] #1 The count updates when any filter changes or the agent list refreshes, and always matches the visible rows
- [x] #2 The window's toolbar (inside the GUI, not the system title bar) shows the number of agent rows currently visible in the table
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Reopened (user): the count belongs inside the window header, not the window manager title bar, which differs per system.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
A label in the toolbar, left of the Filter field, shows 'N agents', N = rows in the table after all filters, recomputed on every render; the window title stays 'hcom-gui'. Verified with contrib/test-hcom-gui test_toolbar_counts_the_shown_rows.
<!-- SECTION:FINAL_SUMMARY:END -->

---
id: TASK-16
title: Po startu agenta v hcom gui nezustava oznaceny radek agenta
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-10-15 13:02'
updated_date: '2026-10-15 13:24'
labels: []
dependencies: []
type: bug
ordinal: 15000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The table selects rows by id agent|instance. Starting an agent turns its catalog row agent| into agent|<instance>, killing does the reverse, so render() no longer finds the selected id and drops the selection. When the selected row disappears, select the agent's row that now exists instead. A plain instance without a catalog agent that disappears stays unselected.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 After a selected stopped agent starts (agent| -> agent|agent), its new row is selected; covered by contrib/test-hcom-gui
- [x] #2 After a selected running instance is killed (agent|x -> agent|), the agent's catalog row is selected; covered by contrib/test-hcom-gui
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
In render(), when the selected iid no longer exists, select the first displayed row with the same agent; test both transitions by swapping the LIVE fixture and refreshing.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
agent_review_hcom: no findings. A plain instance without a catalog agent, or an agent row hidden by Running only, stays unselected.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
render() now selects the first displayed row of the same agent when the selected agent|instance id disappears after start/kill. Verified with contrib/test-hcom-gui (34 OK; both new tests failed before the fix).
<!-- SECTION:FINAL_SUMMARY:END -->

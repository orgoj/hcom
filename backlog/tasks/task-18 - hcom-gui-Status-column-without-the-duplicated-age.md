---
id: TASK-18
title: 'hcom-gui: Status column without the duplicated age'
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-10-15 13:34'
updated_date: '2026-10-15 13:39'
labels: []
dependencies: []
type: bug
ordinal: 17000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Status shows 'listening 2h' while Since shows the same age. Status should show only the state; the age stays in Since.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The Status column shows only the status (e.g. listening), the age only in Since
- [x] #2 Sorting by Status sorts by the state name; covered by contrib/test-hcom-gui
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Test: Status cell shows only the state; sort by status orders by state name. 2. build_rows: status = state only. 3. README if it mentions the status age.
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
build_rows sets status to the state only; Since keeps the age. Sorting by Status orders by state name. Review (agent_review_hcom): no findings. Verified: contrib/test-hcom-gui 41 OK.
<!-- SECTION:FINAL_SUMMARY:END -->

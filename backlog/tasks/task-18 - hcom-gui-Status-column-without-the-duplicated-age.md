---
id: TASK-18
title: 'hcom-gui: Status column without the duplicated age'
status: To Do
assignee: []
created_date: '2026-10-15 13:34'
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
- [ ] #1 The Status column shows only the status (e.g. listening), the age only in Since
- [ ] #2 Sorting by Status sorts by the state name; covered by contrib/test-hcom-gui
<!-- AC:END -->

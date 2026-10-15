---
id: TASK-24
title: 'hcom-gui: text filter for the table'
status: To Do
assignee: []
created_date: '2026-10-15 13:34'
labels: []
dependencies: []
type: feature
ordinal: 23000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
A text field in the toolbar that narrows the table to rows matching the text (case-insensitive) in agent, instance, dir or groups; combines with Running only/Blocked only.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Typing in the filter field shows only matching rows immediately, clearing it shows all rows
- [ ] #2 The filter combines with the checkbox filters and survives the periodic refresh
- [ ] #3 Covered by contrib/test-hcom-gui; README documents it
<!-- AC:END -->

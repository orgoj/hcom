---
id: TASK-38
title: 'hcom-gui: disabled catalog agents shown struck through, hidden by default'
status: To Do
assignee: []
created_date: '2026-10-19 20:07'
updated_date: '2026-10-23 14:11'
labels: []
dependencies: []
type: feature
ordinal: 37000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
hcom-gui fills its table from 'hcom agent list --all --json', which by design (TASK-31) includes disabled catalog agents with "disabled": true. The GUI ignores the field, so a disabled agent (e.g. dabri_dev) looks like an ordinary stopped agent that can be launched.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A 'Hide disabled' filter hides disabled agents' rows; it is on by default
- [ ] #2 The 'Hide disabled' choice persists in ~/.hcom/gui-state.json
- [ ] #3 A disabled catalog agent's row is rendered in a struck-through (overstrike) light-grey font, distinct from enabled stopped agents (plain #888888 grey)
- [ ] #4 contrib/test-hcom-gui covers the struck-through rendering and the filter in both states
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Style decided by the user: struck-through font, because stopped rows are already grey.
<!-- SECTION:NOTES:END -->

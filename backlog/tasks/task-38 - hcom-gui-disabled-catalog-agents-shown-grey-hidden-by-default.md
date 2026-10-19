---
id: TASK-38
title: 'hcom-gui: disabled catalog agents shown grey, hidden by default'
status: To Do
assignee: []
created_date: '2026-10-19 20:07'
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
- [ ] #1 A disabled catalog agent's row is rendered grey, distinct from enabled stopped agents
- [ ] #2 A 'Hide disabled' filter hides disabled agents' rows; it is on by default
- [ ] #3 The 'Hide disabled' choice persists in ~/.hcom/gui-state.json
- [ ] #4 contrib/test-hcom-gui covers the grey rendering and the filter in both states
<!-- AC:END -->

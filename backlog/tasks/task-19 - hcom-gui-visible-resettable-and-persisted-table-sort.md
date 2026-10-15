---
id: TASK-19
title: 'hcom-gui: visible, resettable and persisted table sort'
status: To Do
assignee: []
created_date: '2026-10-15 13:34'
labels: []
dependencies: []
type: feature
ordinal: 18000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Column sort has no indicator, cannot return to the default order (agent, then instance) and is lost on restart.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The sorted column heading shows its direction (▲/▼)
- [ ] #2 A third click on the same heading restores the default order without an indicator
- [ ] #3 Sort column and direction are restored from ~/.hcom/gui-state.json on start
- [ ] #4 Covered by contrib/test-hcom-gui
<!-- AC:END -->

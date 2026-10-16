---
id: TASK-26
title: 'hcom-gui: dialog to choose and order columns'
status: To Do
assignee: []
created_date: '2026-10-16 04:45'
updated_date: '2026-10-16 04:53'
labels: []
dependencies: []
type: feature
ordinal: 25000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Choosing columns today means hand-editing the 'columns' key in ~/.hcom/gui.json (TASK-11). Add a menu entry opening a dialog with a checkbox per column and up/down ordering. The choice is a per-user preference, so it is saved in the GUI state file (~/.hcom/gui-state.json) next to column widths and sort, not in gui.json.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A menu entry opens a dialog listing all columns with a visibility checkbox and up/down buttons to change order
- [ ] #2 Applying the dialog updates the table immediately and saves the selection to gui-state.json; it survives a GUI restart
- [ ] #3 The state selection takes precedence over gui.json 'columns'; without it gui.json 'columns' (or the default set) applies
- [ ] #4 The dialog can reset to the gui.json/default columns; at least one column must stay visible
- [ ] #5 contrib/README.md and contrib/test-hcom-gui cover the dialog and the precedence
<!-- AC:END -->

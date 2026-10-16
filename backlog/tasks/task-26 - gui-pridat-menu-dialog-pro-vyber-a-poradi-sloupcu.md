---
id: TASK-26
title: 'hcom-gui: dialog to choose and order columns'
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-10-16 04:45'
updated_date: '2026-10-16 05:09'
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
- [x] #1 A menu entry opens a dialog listing all columns with a visibility checkbox and up/down buttons to change order
- [x] #2 Applying the dialog updates the table immediately and saves the selection to gui-state.json; it survives a GUI restart
- [x] #3 The state selection takes precedence over gui.json 'columns'; without it gui.json 'columns' (or the default set) applies
- [x] #4 The dialog can reset to the gui.json/default columns; at least one column must stay visible
- [x] #5 contrib/README.md and contrib/test-hcom-gui cover the dialog and the precedence
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
No menubar in hcom-gui: the entry is a Columns… toolbar button; checkboxes are ☑/☐ Listbox items toggled by Space/double-click.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added Columns… dialog to contrib/hcom-gui: toggle (Space/double-click), reorder (Up/Down, Ctrl+Up/Down), Reset, at least one visible. Choice saved immediately to gui-state.json and takes precedence over gui.json columns; hiding the sort column resets the sort. Documented in script header and contrib/README.md. Verified with contrib/test-hcom-gui (62 OK); reviewed OK by agent_review_hcom.
<!-- SECTION:FINAL_SUMMARY:END -->

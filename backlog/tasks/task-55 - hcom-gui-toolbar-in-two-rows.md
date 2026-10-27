---
id: TASK-55
title: 'hcom-gui: toolbar in two rows'
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-10-27 19:41'
updated_date: '2026-10-27 20:50'
labels: []
dependencies: []
type: feature
ordinal: 56000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The toolbar no longer fits on the screen. Split it into two rows: the first holds the command buttons (Start ... commands, Columns, Refresh), the second the rest: row count, Filter field and the filter checkboxes (Hide non-persistent, Hide disabled, Blocked only, Running only, Empty last).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Keys, button states and the gui-state.json filters work as before
- [x] #2 contrib/test-hcom-gui passes and checks the two rows; contrib/README.md describes the layout if it mentions the toolbar placement
- [x] #3 The button row wraps automatically to the window width: no button is cut off, a row never starts or ends with a separator, and resizing the window reflows the buttons
- [x] #4 gui.json toolbar_breaks (list of button labels) starts a new button row after each named button; an unknown label is reported in the log
- [x] #5 The toolbar's button rows hold only buttons; the row below them holds the row count, the Filter field and the checkboxes
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Reopened at the user's request: the button row is still too wide on a portrait monitor; adds automatic wrapping and configurable breaks (AC 4, 5).

Reopened part: buttons wrap to the window width (_wrap_toolbar on <Configure>, row frames, separators dropped at row edges, Columns/Refresh at the end of the flow); gui.json toolbar_breaks (button labels incl. Columns, Refresh; unknown logged; trailing break adds no row). 143 tests OK, reviewed by agent_review.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Toolbar split into two rows: buttons (incl. Columns, Refresh) on the first, row count, Filter field and filter checkboxes on the second in the previous order. Variables, keys and gui-state.json unchanged. New test checks both rows; 134 tests OK. README updated. Reviewed by agent_review.
<!-- SECTION:FINAL_SUMMARY:END -->

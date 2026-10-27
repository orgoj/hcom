---
id: TASK-56
title: 'hcom-gui: Screen key bar (Inject and special keys)'
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-10-27 22:21'
updated_date: '2026-10-27 22:26'
labels: []
dependencies: []
type: feature
ordinal: 57000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Goal: from the Screen tab, answer TUI prompts that need keys a text field cannot carry. A bar at the bottom of the Screen tab holds the Inject button (moved from the toolbar) and buttons Esc, Up, Down, Left, Right, Tab, Shift+Tab, Enter; each runs hcom term inject <instance> <escape sequence> (Enter: --enter). No binary change.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The Screen tab has a bottom bar with Inject and the key buttons; Inject is no longer in the toolbar
- [x] #2 Each key button runs hcom term inject <instance> with its sequence (Esc \e, arrows \e[A/B/C/D, Tab \t, Shift+Tab \e[Z, Enter --enter), enabled only for a running instance
- [x] #3 contrib/test-hcom-gui covers the bar; contrib/README.md and the docstring document it
- [x] #4 Buttons 1, 2, 3 inject the digit alone (numbered menu choice)
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Bar under the Screen tab: Inject (moved from the toolbar) and key buttons Esc, arrows (CSI), Tab, Shift+Tab, Enter (--enter), 1, 2, 3, each running hcom term inject <instance> <key>, enabled for running instances. Log escapes control characters. Arrow limitation (ESC O-only programs) documented. 4 new tests, 146 OK. Reviewed by agent_review.
<!-- SECTION:FINAL_SUMMARY:END -->

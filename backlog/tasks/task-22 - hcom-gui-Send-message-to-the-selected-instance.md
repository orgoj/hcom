---
id: TASK-22
title: 'hcom-gui: Send message to the selected instance'
status: To Do
assignee: []
created_date: '2026-10-15 13:34'
labels: []
dependencies: []
type: feature
ordinal: 21000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
A toolbar button that sends an hcom message to the selected running instance through hcom send, with a multi-line text field like the Start prompt (Shift+Enter newline, Enter/keypad Enter sends).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Send is enabled only for a running instance and runs hcom send @<instance> with the entered text; an empty text sends nothing
- [ ] #2 The message dialog is multi-line with Shift+Enter for a newline and Enter/keypad Enter to send
- [ ] #3 Covered by contrib/test-hcom-gui; README documents the button
<!-- AC:END -->

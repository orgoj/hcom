---
id: TASK-17
title: 'hcom-gui: open_with buttons run during a command and report failures'
status: To Do
assignee: []
created_date: '2026-10-15 13:34'
labels: []
dependencies: []
type: bug
ordinal: 16000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
open_with buttons (Zed, gitg, gitk, Backlog, ...) and their shortcuts are ignored while an hcom command runs (busy, e.g. a Start that waits for launch readiness), although they start independent programs. Their stdout/stderr go to DEVNULL, so a program that exits at once with an error leaves only the $ line in the log.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 An open_with button and its key shortcut start their program while another hcom command is running
- [ ] #2 An open_with program that exits non-zero shortly after start (e.g. within a few seconds) writes its exit code and stderr to the log
- [ ] #3 Both behaviours are covered by contrib/test-hcom-gui
<!-- AC:END -->

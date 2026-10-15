---
id: TASK-17
title: 'hcom-gui: open_with buttons run during a command and report failures'
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-10-15 13:34'
updated_date: '2026-10-15 13:38'
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
- [x] #1 An open_with button and its key shortcut start their program while another hcom command is running
- [x] #2 An open_with program that exits non-zero shortly after start (e.g. within a few seconds) writes its exit code and stderr to the log
- [x] #3 Both behaviours are covered by contrib/test-hcom-gui
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Test: detach button runs while busy; failing detach program logs exit code+stderr. 2. run_button: detach bypasses busy. 3. Detach Popen captures stderr; watcher thread logs non-zero exit within a few seconds. 4. README/docstring.
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
open_with buttons and keys bypass the busy guard and stay enabled during a command. Detached programs write stderr to an unlinked temp file (no SIGPIPE after the GUI stops watching); a non-zero exit within DETACH_WATCH_SECONDS=5 logs '<label>: <stderr>[exit N]'. Temp file failure logs ERROR. Review (agent_review_hcom): low finding fixed, timeout branch tested. Verified: contrib/test-hcom-gui 39 OK.
<!-- SECTION:FINAL_SUMMARY:END -->

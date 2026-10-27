---
id: TASK-54
title: 'hcom-gui: Inject button (hcom term inject)'
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-10-27 19:40'
updated_date: '2026-10-27 19:47'
labels: []
dependencies: []
type: feature
ordinal: 55000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Goal: type text straight into the selected agent's terminal from contrib/hcom-gui, e.g. to answer a prompt or confirm a dialog that a queued hcom send cannot reach. New toolbar button Inject next to Send runs the existing CLI: hcom term inject <instance> <text> --enter (the GUI holds no hcom logic). Only PTY instances support it; hcom's own error is logged.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The toolbar has an Inject button next to Send, enabled only for a row with a running instance
- [x] #2 The command output and a failure (e.g. a non-PTY instance) appear in the Log like other buttons
- [x] #3 contrib/test-hcom-gui covers the button; contrib/README.md and the hcom-gui docstring document it
- [x] #4 Inject asks for the text and an Enter checkbox (checked by default) and runs hcom term inject <instance> <text>, with --enter only when the box is checked; an empty text runs nothing
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Inject button after Send (running instances): asks for a text and an Enter checkbox (checked by default), runs hcom term inject <instance> <text> [--enter]; empty text runs nothing via the new button key required (hcom term inject has no --). Output and failures go to the Log. 6 new tests, 140 OK. README and docstring updated. Reviewed by agent_review.
<!-- SECTION:FINAL_SUMMARY:END -->

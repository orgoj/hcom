---
id: TASK-22
title: 'hcom-gui: Send message to the selected instance'
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-10-15 13:34'
updated_date: '2026-10-15 14:02'
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
- [x] #1 Send is enabled only for a running instance and runs hcom send @<instance> with the entered text; an empty text sends nothing
- [x] #2 The message dialog is multi-line with Shift+Enter for a newline and Enter/keypad Enter to send
- [x] #3 Covered by contrib/test-hcom-gui; README documents the button
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Tests: Send disabled on a catalog row, enabled on a running one; runs hcom send -b @<instance> -- <text>; empty text runs nothing; dialog for message is multi-line (Shift+Enter, Enter/KP_Enter). 2. BUTTONS: Send (when running) after Attach; an {ask:X} right after -- is required; ask() treats message like prompt; dialog title names the instance. 3. README + docstring.
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Send button (when running) runs hcom send -b @<instance> -- <text>; {ask:X} after a bare -- is required, an empty text logs and runs nothing; the message field is multi-line like the Start prompt; the dialog names the instance. README documents it. Review (agent_review_hcom): verification gap closed. Verified: contrib/test-hcom-gui 53 OK.
<!-- SECTION:FINAL_SUMMARY:END -->

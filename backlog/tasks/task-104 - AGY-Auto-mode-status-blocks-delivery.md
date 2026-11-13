---
id: TASK-104
title: AGY Auto mode status blocks delivery
status: Done
assignee: []
created_date: '2026-11-13T06:00:00Z'
updated_date: '2026-11-13T06:03:00Z'
labels: []
dependencies: []
ordinal: 211000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Antigravity shows 'Auto mode: edits & safe commands auto-approved (shift+tab to cycle)' in the prompt row; the screen parser treats it as typed input (tui:prompt-has-text) and holds delivery (seen: #352083 to agent_architect_nuntora_435). Treat it like the Accept-edits status (cursor parked after '> ').
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Auto mode status at prompt cursor reads as empty prompt
- [x] #2 Same text with cursor after it stays protected input
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Antigravity Auto mode status added to ANTIGRAVITY_MODE_STATUSES (src/pty/screen.rs); the existing test now covers both statuses. 0.7.30-orgoj.42.
<!-- SECTION:FINAL_SUMMARY:END -->

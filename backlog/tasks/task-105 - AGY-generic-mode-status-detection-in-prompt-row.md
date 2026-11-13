---
id: TASK-105
title: AGY generic mode-status detection in prompt row
status: Done
assignee: []
created_date: '2026-11-13T08:15:00Z'
updated_date: '2026-11-13T08:19:00Z'
labels: []
dependencies: []
ordinal: 213000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Exact-string matching of agy mode statuses (TASK-104) breaks on every new mode text. Treat any prompt-row text ending with '(shift+tab to cycle)' as empty when the cursor is parked right after '> '.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Unknown mode status ending with '(shift+tab to cycle)' at prompt cursor reads as empty
- [x] #2 Same text with cursor after it stays protected input
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Prompt-row text ending '(shift+tab to cycle)' with the cursor right after '> ' reads as an empty prompt (src/pty/screen.rs). The test covers both known statuses and an unknown one. Review finding rejected (see the review:rejected task). 0.7.30-orgoj.43.
<!-- SECTION:FINAL_SUMMARY:END -->

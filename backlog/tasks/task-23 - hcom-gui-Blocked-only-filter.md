---
id: TASK-23
title: 'hcom-gui: Blocked only filter'
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-10-15 13:34'
updated_date: '2026-10-15 14:04'
labels: []
dependencies: []
type: feature
ordinal: 22000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
A 'Blocked only' checkbox next to 'Running only' that shows only blocked instances; persisted in gui-state.json like Running only.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 With Blocked only checked the table lists only instances in blocked status
- [x] #2 The checkbox state is restored on start
- [x] #3 Covered by contrib/test-hcom-gui; README documents it
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Tests: with Blocked only only blocked instances are listed; state saved and restored. 2. blocked_only BooleanVar + checkbox next to Running only; render filter; _restore_state/_on_close. 3. README + docstring.
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Blocked only checkbox next to Running only: render lists only rows with state blocked (AND with Running only); blocked_only saved in gui-state.json and restored. README and docstring updated. Review (agent_review_hcom): verification gap closed. Verified: contrib/test-hcom-gui 54 OK.
<!-- SECTION:FINAL_SUMMARY:END -->

---
id: TASK-19
title: 'hcom-gui: visible, resettable and persisted table sort'
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-10-15 13:34'
updated_date: '2026-10-15 13:40'
labels: []
dependencies: []
type: feature
ordinal: 18000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Column sort has no indicator, cannot return to the default order (agent, then instance) and is lost on restart.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The sorted column heading shows its direction (▲/▼)
- [x] #2 A third click on the same heading restores the default order without an indicator
- [x] #3 Sort column and direction are restored from ~/.hcom/gui-state.json on start
- [x] #4 Covered by contrib/test-hcom-gui
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Tests: heading shows ▲/▼, third click resets (no indicator, default order), sort restored from state file. 2. sort_by cycles asc→desc→off and updates heading texts. 3. _on_close saves sort_col/sort_reverse; _restore_state restores if the column exists. 4. README.
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Heading clicks cycle ascending (▲), descending (▼), default order; render() marks the sorted heading. sort_col/sort_reverse saved in gui-state.json and restored when the column is still shown. Review (agent_review_hcom): no findings. Verified: contrib/test-hcom-gui 43 OK.
<!-- SECTION:FINAL_SUMMARY:END -->

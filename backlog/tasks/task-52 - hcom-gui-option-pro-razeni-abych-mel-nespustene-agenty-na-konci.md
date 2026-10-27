---
id: TASK-52
title: 'hcom-gui: option pro razeni abych mel nespustene agenty na konci'
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-10-27 16:14'
updated_date: '2026-10-27 19:19'
labels: []
dependencies: []
type: feature
ordinal: 53000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Sorting by a column puts rows with an empty cell first (ascending) or between values, e.g. by Instance the stopped catalog rows (empty instance) come first. A checkbox should keep empty cells at the end for any sort column.

In scope: toolbar checkbox 'Empty last', default on, persisted in gui-state.json; README/docstring.
Out of scope: the default order (no sort column), which stays unchanged.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Toolbar checkbox 'Empty last' is on by default and its state is kept in ~/.hcom/gui-state.json
- [x] #2 While it is on and a sort column is chosen, rows whose cell in that column is empty come after all others, ascending and descending
- [x] #3 While it is off, sorting is unchanged from before
- [x] #4 contrib/test-hcom-gui covers on (asc and desc), off and restore; contrib/README.md documents it
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
BooleanVar empty_last (state key empty_last, default True), Checkbutton in toolbar calling render. render(): after sorting by sort_col, stable-partition rows with r[sort_col] == '' to the end when on. Tests, README, docstring.
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Toolbar checkbox Empty last (default on, kept in gui-state.json) moves rows with an empty cell in the sort column to the end, ascending and descending; default order unchanged. Verified with contrib/test-hcom-gui (123 tests OK), reviewed by agent_review.
<!-- SECTION:FINAL_SUMMARY:END -->

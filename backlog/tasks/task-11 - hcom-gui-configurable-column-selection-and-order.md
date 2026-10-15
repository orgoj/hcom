---
id: TASK-11
title: 'hcom-gui: configurable column selection and order'
status: To Do
assignee: []
created_date: '2026-10-15 09:46'
labels: []
dependencies: []
type: feature
ordinal: 11000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
As the column count grows, let the user choose which columns are shown and in which order via ~/.hcom/gui.json, defaulting to the current set.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 gui.json "columns" (list of column keys) selects and orders the visible columns
- [ ] #2 Without the key the GUI shows the default columns; unknown keys are reported in the log and skipped
- [ ] #3 Saved column widths keep working for the configured columns
- [ ] #4 contrib/README.md and contrib/test-hcom-gui cover the option
<!-- AC:END -->

---
id: TASK-11
title: 'hcom-gui: configurable column selection and order'
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-10-15 09:46'
updated_date: '2026-10-15 10:32'
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
- [x] #1 gui.json "columns" (list of column keys) selects and orders the visible columns
- [x] #2 Without the key the GUI shows the default columns; unknown keys are reported in the log and skipped
- [x] #3 Saved column widths keep working for the configured columns
- [x] #4 contrib/README.md and contrib/test-hcom-gui cover the option
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added gui.json "columns" to contrib/hcom-gui: selects and orders columns, default all, unknown keys logged and skipped; widths restore for visible columns (hidden ones are not kept on close). Documented in the script header and contrib/README.md. Verified with contrib/test-hcom-gui (26 OK); reviewed OK by agent_review_hcom.
<!-- SECTION:FINAL_SUMMARY:END -->

---
id: TASK-24
title: 'hcom-gui: text filter for the table'
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-10-15 13:34'
updated_date: '2026-10-15 14:11'
labels: []
dependencies: []
type: feature
ordinal: 23000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
A text field in the toolbar that narrows the table to rows matching the text (case-insensitive) in agent, instance, dir or groups; combines with Running only/Blocked only.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Typing in the filter field shows only matching rows immediately, clearing it shows all rows
- [x] #2 The filter combines with the checkbox filters and survives the periodic refresh
- [x] #3 Covered by contrib/test-hcom-gui; README documents it
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Tests: typing filters by agent/instance/dir/groups case-insensitively, clearing shows all, combines with Running only, survives refresh; typing in the field does not fire table shortcuts. 2. filter StringVar + Entry in toolbar, trace -> render; render skips non-matching rows. 3. README + docstring.
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Toolbar Filter entry (StringVar trace -> render) narrows the table to rows whose agent, instance, dir (raw or ~ form) or groups contain the text, case-insensitive; AND with Running only/Blocked only; kept across refresh, not persisted. README and docstring updated. Review (agent_review_hcom, agy): approved; tilde match test added. Verified: contrib/test-hcom-gui 56 OK.
<!-- SECTION:FINAL_SUMMARY:END -->

---
id: TASK-14
title: Definovatelne zkratky na hcom gui commandy
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-10-15 12:56'
updated_date: '2026-10-15 13:06'
labels: []
dependencies: []
type: feature
ordinal: 13000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
hcom-gui toolbar buttons (open_with and commands) can be run with a keyboard shortcut defined by "key" in ~/.hcom/gui.json. Wanted locally: b Backlog, z Zed, g gitg, h gitk --all.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 An open_with entry with "key" runs its command for the selected row when the key is pressed in the table, covered by contrib/test-hcom-gui
- [x] #2 contrib/README.md and the hcom-gui docstring document "key" for open_with and commands with an example
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Key binding already exists in _create_widgets for every button; add a regression test and document it.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Status left at Idea during implementation (process error, reported to agent_coach); set retroactively. Live ~/.hcom/gui.json (machine-only) got keys b/z/g/h. agent_review_hcom: no findings.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Shortcuts already worked via the existing key binding; added test_open_with_key_opens_selected_dir and documented key for open_with/commands in contrib/README.md and the hcom-gui docstring. Verified with contrib/test-hcom-gui (27 OK).
<!-- SECTION:FINAL_SUMMARY:END -->

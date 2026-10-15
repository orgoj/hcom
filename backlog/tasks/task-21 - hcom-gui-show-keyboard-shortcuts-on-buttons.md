---
id: TASK-21
title: 'hcom-gui: show keyboard shortcuts on buttons'
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-10-15 13:34'
updated_date: '2026-10-15 13:46'
labels: []
dependencies: []
type: feature
ordinal: 20000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Shortcuts from "key" (gui.json open_with/commands, Delete on Kill) are invisible in the toolbar.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A button with a key shows it in its label, e.g. 'Zed (z)', 'Kill (Del)'
- [x] #2 Covered by contrib/test-hcom-gui; README describes the label
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Test: open_with Zed key z -> 'Zed (z)', Kill -> 'Kill (Del)', <Control-F2> -> 'Ctrl+F2', no key -> plain label. 2. key_label() turning a Tk sequence into a short text; _create_widgets appends it. 3. README + docstring.
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Buttons with a key show it in the label via key_label() (Delete->Del, Control-/Shift-/Alt- -> Ctrl+/Shift+/Alt+): 'Zed (z)', 'Kill (Del)', 'Refresh (F5)'. README and docstring describe it. Review (agent_review_hcom): Refresh finding fixed. Verified: contrib/test-hcom-gui 49 OK.
<!-- SECTION:FINAL_SUMMARY:END -->

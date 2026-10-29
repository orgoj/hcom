---
id: TASK-63
title: 'hcom-gui: Restart GUI toolbar button'
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-10-29 07:53'
updated_date: '2026-10-29 07:55'
labels: []
dependencies: []
priority: low
type: enhancement
ordinal: 64000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
User request: a toolbar button next to Refresh that restarts hcom-gui (new code and gui.json) without asking; no key.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A 'Restart GUI' toolbar button after Refresh saves gui-state.json and re-executes hcom-gui with its original interpreter and arguments, without a confirmation
- [x] #2 toolbar_breaks accepts 'Restart GUI'; contrib/test-hcom-gui covers the button; contrib/README.md and the docstring document it
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Restart GUI toolbar button after Refresh: saves gui-state.json, then os.execv(sys.executable, sys.orig_argv) without asking; accepted by toolbar_breaks. Tests: contrib/test-hcom-gui 160 OK (new case, toolbar layout cases updated). Docs: README, docstring. Small change, no review.
<!-- SECTION:FINAL_SUMMARY:END -->

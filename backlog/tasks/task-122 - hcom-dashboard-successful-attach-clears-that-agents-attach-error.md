---
id: TASK-122
title: 'hcom-dashboard: successful attach clears that agent''s attach error'
status: Done
assignee:
  - '@hcom_dev__fix'
created_date: '2026-11-19T09:45:00Z'
updated_date: '2026-11-19T10:04:00Z'
labels: []
dependencies: []
priority: medium
type: bug
ordinal: 233000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
A failed Attach (Focus) (e.g. double-click while the agent was still starting: herdr agent target not found) leaves the banner 'Attach to X failed' even after a later attach of X succeeds. Screen/Transcript already clear their stale error via clear_stale_error (contrib/hcom-dashboard/src/app.rs); AttachFinished Ok only logs.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A successful attach of agent X removes a shown 'Attach to X failed' error
- [x] #2 Errors of other kinds or other agents stay
- [x] #3 Test covers both
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
AttachFinished Ok now drops the shown 'Attach to X failed' error of the same agent (clear_error_starting, shared with clear_stale_error). Verified by test_task_122_successful_attach_clears_its_stale_error; dashboard fmt/clippy clean, suite green except the known load-sensitive timing tests (load ~14 on 4 cores; they fail intermittently with and without the change).
<!-- SECTION:FINAL_SUMMARY:END -->

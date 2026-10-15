---
id: TASK-7
title: 'hcom-gui: status age and agent age as sortable columns'
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-10-15 09:46'
updated_date: '2026-10-15 10:23'
labels: []
dependencies: []
type: feature
ordinal: 7000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
hcom list --json gives status_age_seconds and created_at; the GUI only folds the status age into the Status text ('listening 3h'), so rows cannot be sorted by how long an agent has been idle/blocked or how long it has been running. Add two columns: Since (age of the current status) and Age (time since the instance was created).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A Since column shows the status age (e.g. 3h, 10m) for running instances
- [x] #2 An Age column shows the time since the instance was created (created_at) for running instances
- [x] #3 Both are empty for catalog-only rows and sort by the numeric age, not the text
- [x] #4 contrib/test-hcom-gui covers both columns
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added Since (status_age_seconds) and Age (now - created_at) columns to contrib/hcom-gui; values stay in seconds for numeric sort and are formatted by cell(). Verified with contrib/test-hcom-gui (16 OK); reviewed OK by agent_review_hcom.
<!-- SECTION:FINAL_SUMMARY:END -->

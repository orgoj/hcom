---
id: TASK-10
title: 'hcom-gui: terminal location column (Herdr workspace/tab)'
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-10-15 09:46'
updated_date: '2026-10-15 10:31'
labels: []
dependencies: []
type: feature
ordinal: 10000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
launch_context carries herdr_workspace and herdr_tab (and pane_id); showing them lets the user find an agent's pane quickly.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A Where column shows workspace/tab for instances launched in Herdr
- [x] #2 Instances without Herdr context show an empty value
- [x] #3 contrib/test-hcom-gui covers the column
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added Where column (Herdr workspace/tab from launch_context) to contrib/hcom-gui. Verified with contrib/test-hcom-gui (24 OK) and live rows; reviewed OK by agent_review_hcom.
<!-- SECTION:FINAL_SUMMARY:END -->

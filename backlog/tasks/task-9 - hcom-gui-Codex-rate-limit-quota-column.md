---
id: TASK-9
title: 'hcom-gui: Codex rate-limit quota column'
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-10-15 09:46'
updated_date: '2026-10-15 10:29'
labels: []
dependencies: []
type: feature
ordinal: 9000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Codex rollout token_count events carry rate_limits.primary.used_percent and window_minutes (the 5h quota). Show it per Codex instance so the user sees quota exhaustion before an agent stalls.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A Quota column shows the latest primary used_percent for running Codex instances
- [x] #2 Non-Codex rows and transcripts without rate_limits show an empty value
- [x] #3 contrib/test-hcom-gui covers the column
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added Quota column to contrib/hcom-gui from the latest Codex token_count rate_limits.primary.used_percent; transcript parsing generalized to transcript_usage(). Verified with contrib/test-hcom-gui (23 OK) and a live run (4 Codex instances); reviewed OK by agent_review_hcom.
<!-- SECTION:FINAL_SUMMARY:END -->

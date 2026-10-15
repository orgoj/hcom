---
id: TASK-12
title: Listening status age is always 0s
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-10-15 12:28'
updated_date: '2026-10-15 12:33'
labels: []
dependencies: []
type: bug
ordinal: 12000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
get_instance_status (src/instance_lifecycle.rs) zeroes age_seconds for every listening agent with a fresh heartbeat, although status_time holds when it went idle. So hcom list --json reports status_age_seconds 0, hcom-gui shows 'listening 0s' and Since 0s, the 'since X' suffix of hcom list never appears, and the subagent timeout countdown never runs.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 hcom list --json reports status_age_seconds as the time since a listening agent went idle
- [x] #2 hcom list still shows 'now' as the leading age of a listening agent and appends 'since X' after 60s idle
- [x] #3 Unit test covers the idle age of a listening agent with a fresh heartbeat
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Listening with fresh heartbeat (or wake grace) keeps the status_time age in age_seconds; age_string stays 'now' for listening. 2. Unit test. 3. Standard finishing: bump, fmt/clippy/test, review, commit, release build.
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
get_instance_status keeps the status_time age for live listeners (age_string stays 'now', remote listeners stay 0). Verified: unit test, cargo test --bin hcom, cli_smoke list, live hcom list --json and 'since X' suffix.
<!-- SECTION:FINAL_SUMMARY:END -->

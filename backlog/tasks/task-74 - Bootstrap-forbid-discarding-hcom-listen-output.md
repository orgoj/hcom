---
id: TASK-74
title: 'Bootstrap: forbid discarding hcom listen output'
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-10-31 20:11'
updated_date: '2026-10-31 20:16'
labels: []
dependencies: []
priority: medium
type: bug
ordinal: 75000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
hcom listen consumes the messages it returns. heca_main used 'hcom listen 180 --name heca_main >/dev/null 2>&1' as a sleep in a background loop and lost request #259520. The bootstrap waiting rules recommend listen instead of sleep without saying it consumes messages.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Bootstrap waiting rules (auto and adhoc delivery) say hcom listen consumes messages and forbid redirecting its output to /dev/null or running it in background loops
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Bootstrap waiting rules (auto, adhoc and subagent) state that hcom listen consumes the messages it prints and forbid >/dev/null and background loops. Verified: fmt/clippy clean, cargo test --bin hcom ok (one known flaky idle_kill test passed in isolation), test_delivery_section_follows_launch_state asserts the rule for every integration.
<!-- SECTION:FINAL_SUMMARY:END -->

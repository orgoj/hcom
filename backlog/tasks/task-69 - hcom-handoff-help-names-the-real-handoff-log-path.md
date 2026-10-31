---
id: TASK-69
title: hcom handoff --help names the real handoff log path
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-10-31 07:59'
updated_date: '2026-10-31 08:01'
labels: []
dependencies: []
priority: low
type: bug
ordinal: 70000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Help says logs/handoff-<name>.log; the log is written to <hcom dir>/.tmp/logs/handoff-<name>.log (src/commands/handoff.rs, paths::LOGS_DIR).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 hcom handoff --help names .tmp/logs/handoff-<name>.log in the hcom data directory
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Help line now names ~/.hcom/.tmp/logs/handoff-<name>.log (matches handoff.rs + paths::LOGS_DIR). Verified with fmt, clippy, cargo test --bin hcom (only known-flaky idle_kill test failed, passes alone), cli_smoke handoff.
<!-- SECTION:FINAL_SUMMARY:END -->

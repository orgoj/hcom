---
id: TASK-72
title: Neutral example names in tests instead of local agent names
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-10-31 09:42'
updated_date: '2026-10-31 09:45'
labels: []
dependencies: []
priority: low
type: chore
ordinal: 73000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Unit tests (src/**), tests/cli_smoke.rs and contrib/test-hcom-gui use local agent and project names (wdt, wdt_main, wdt_all, ansible-wdt, WDT) as fixtures. Replace them with neutral names (ops, ops_main, ops_all, ops-repo) consistently, without changing what the tests check. HCOM_DEV_ROOT is a real env var and stays.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 rg -i 'wdt' over src, tests and contrib finds nothing
- [x] #2 Renamed tests keep their assertions and pass (cargo test --bin hcom, affected cli_smoke tests, contrib/test-hcom-gui)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
GUI fixture uses web/web_all instead of ops: the group menu sorts alphabetically and the tests rely on the group sorting after 'persistent', as wdt_all did. gtm_cli also renamed to api_cli. Test-only change: no non-test code, no version bump or release build.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Replaced wdt/ansible-wdt/WDT/gtm_cli fixtures with ops/ops-repo/web/api_cli in unit tests, cli_smoke and contrib/test-hcom-gui. Verified: cargo fmt --check, clippy zero warnings, cargo test --bin hcom (one unrelated flaky codex test passed in isolation), the 5 touched cli_smoke tests, contrib/test-hcom-gui 168 OK.
<!-- SECTION:FINAL_SUMMARY:END -->

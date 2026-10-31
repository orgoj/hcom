---
id: TASK-73
title: 'agy: no --add-dir for external bundles (run_command cwd)'
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-10-31 16:17'
updated_date: '2026-10-31 16:22'
labels: []
dependencies: []
priority: medium
type: bug
ordinal: 74000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
AGY 1.4.12 run_command sends no Cwd and runs in the lexicographically first workspace dir. hcom passed --add-dir <bundle> for agy/antigravity, and ~/.hcom/agents/<x> sorts before typical project dirs, so commands ran in the bundle instead of the agent dir (knowledge ran git status in ~/.hcom). Fix approved by agent_coach (bigboss): no --add-dir for agy/antigravity; the bundle stays reachable as outside-workspace files under --dangerously-skip-permissions.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 agy and antigravity launches with an external bundle pass no --add-dir
- [x] #2 Docs, built-in help and skill reference state the agy exception and why
- [x] #3 Unit tests cover the agy/antigravity case
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
apply_bundle_access passes no --add-dir for agy/antigravity; help, docs/agent.md and skill reference state the exception and why. Verified: cargo fmt/clippy clean, cargo test --bin hcom 2895 ok, cli_smoke agent_ 66 ok, hcom agent show knowledge renders no --add-dir. AGY tests: without --add-dir run_command cwd = launch dir and bundle writes work under --dangerously-skip-permissions.
<!-- SECTION:FINAL_SUMMARY:END -->

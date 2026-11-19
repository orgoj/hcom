---
id: TASK-127
title: idle_kill worker spawn fails after the hcom binary was replaced
status: Done
assignee:
  - '@hcom_dev__fix'
created_date: '2026-11-19T14:31:00Z'
updated_date: '2026-11-19T14:33:00Z'
labels: []
dependencies: []
type: bug
ordinal: 238000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
idle_kill::sweep_throttled spawns the detached worker via std::env::current_exe(). A long-running hcom process (e.g. an agent's PTY wrapper) whose binary was replaced by a release build gets '<path> (deleted)' and every spawn fails: log idle_kill.spawn_failed name=wdt_senior exe=/home/michael/projects/hcom/target/release/hcom (deleted), 2026-11-19. User decision: the kill worker is started through 'hcom' from PATH.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 idle_kill starts its worker as 'hcom' resolved from PATH, not from current_exe
- [x] #2 Existing idle_kill unit and cli_smoke tests pass
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
sweep_throttled spawns the idle-kill worker as 'hcom' from PATH instead of current_exe(), which is a deleted file once a release build replaced the binary under a long-running process. Verified: cargo test --bin hcom (2934 ok), cli_smoke idle_kill (3 ok; tests put the test binary first in PATH), fmt, clippy.
<!-- SECTION:FINAL_SUMMARY:END -->

---
id: TASK-59
title: Verify hcom.db write-lock waits after all agents run synchronous=NORMAL
status: To Do
assignee: []
created_date: '2026-10-28 09:55'
updated_date: '2026-10-28 09:56'
labels: []
dependencies:
  - TASK-58
priority: medium
type: task
ordinal: 60000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
TASK-58 set synchronous=NORMAL; agents started before the 0.7.30-orgoj.22 deploy keep the old binary and still fsync on every commit. Once every running agent has been restarted, measure live. Baseline 2026-10-28 08:12-08:43Z: hooks (without poll/hermes-read) >1s: 5 of 355, max 5250ms; WAL write lock holds >0.5s: 12 in 30 min, all hcom pty processes.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every running hcom pty process was started after the TASK-58 deploy (no /proc/<pid>/exe '(deleted)' from before it)
- [ ] #2 A 30-min WAL write-lock sample shows fewer holds >0.5s than the baseline of 12
- [ ] #3 Hook handler times (without poll/hermes-read) over a comparable window show fewer >1s than the baseline of 5 of 355, and no hook >5s
- [ ] #4 Result recorded in the task's final summary; if not improved, the next suspected cause is named
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Method used for the baseline: sample /proc/locks every 20ms for a POSIX WRITE lock at byte 120 of ~/.hcom/hcom.db-shm (SQLite WAL write lock), log holder pid, cmdline and hold time >0.5s; hook times from claude.dispatch.timing handler_ms in ~/.hcom/.tmp/logs/hcom.log.
<!-- SECTION:NOTES:END -->

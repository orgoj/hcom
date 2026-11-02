---
id: TASK-77.12
title: 'Doplnění sběru context window, quota a idle stavu do hcom-graph'
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-11-01 16:42'
updated_date: '2026-11-02 19:05'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: medium
type: feature
ordinal: 89000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Doplnění chybějících sloupců z hcom-gui do hcom-graph: context window fill (čtení Claude/Codex transcript usage), rate-limit quota (Codex) a idle klasifikátor podle screen/activity. V TASK-77.4 zůstaly tyto sloupce prázdné.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Sběr context window fill pro Claude a Codex
- [x] #2 Sběr quota pro Codex
- [x] #3 Idle stav podle screen/activity
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Context (Claude, Codex) and quota (Codex) cells from the last 1 MiB of the instance transcript (new usage.rs, port of hcom-gui transcript_usage; symlink/non-regular paths rejected at lstat and at open via O_NOFOLLOW|O_NONBLOCK; cached by mtime/size), numeric sort keys. Idle column (new idle.rs): listening periods per hcom-gui idle_periods, queued after idle_seconds while the column is shown, classified by contrib/idle-classifier/idle_check.py in its own thread and process group (one run at a time, max 32 names, 180 s timeout, stdout/stderr caps, group killed on timeout/overflow/drop and after leader exit via waitid WNOWAIT); errors logged as idle_check's own line only. Out of scope: needs_user highlight, right-click --label. Tests: usage parsing/cache/FIFO/symlink, period logic, output/error parsing, lifecycle timeout/drop/overflow/leftover, App idle integration. Gates fmt, clippy 0, cargo test ok. Reviewed by agent_review and agent_security.
<!-- SECTION:FINAL_SUMMARY:END -->

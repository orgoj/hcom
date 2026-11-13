---
id: TASK-110
title: Audit timeouts whose expiry changes state other logic depends on
status: To Do
assignee: []
created_date: '2026-11-13T20:05:00Z'
labels: []
dependencies: []
priority: medium
type: bug
ordinal: 219000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
TASK-109: CONNECT_GRACE (15 s, src/hooks/claude_mod.rs, upstream 1ef0c822) only gates a diagnostic, yet on a slow Claude start (21 s) report_health overwrote status_context 'start', which is_fresh_claude_process_placeholder (src/hooks/claude.rs) needs for the SessionStart bind; the session never bound and messages stayed queued. Fixed in ced23ed5 for that one path. User: short timeouts must not break functions. Review every timeout/grace in the binary (delivery loop, launch readiness and launch_blocked in src/delivery.rs, mod HEALTH_TIMEOUT, launcher readiness wait, hook waits) for expiry that writes state (status, status_context, bindings, cursors) read by binding or delivery logic. Known: launch_blocked is accepted by the SessionStart predicate (upstream 8e0e1c6a). Keep upstream constants unless a value itself breaks behavior; fixes go upstream-mergeable.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every timeout in the binary is listed in the task notes with what its expiry writes and who reads it
- [ ] #2 Each timeout whose expiry can break a later binding or delivery has a fix with a regression test, or a recorded reason why not
<!-- AC:END -->

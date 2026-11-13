---
id: TASK-109
title: >-
  Slow Claude start: mod health clobbers launch context, SessionStart never
  binds
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-11-13T19:13:00Z'
updated_date: '2026-11-13T19:16:00Z'
labels: []
dependencies: []
priority: high
type: bug
ordinal: 218000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
dippy_dev__allow (claude, native mod delivery) never received messages from bin. Claude took 21 s to reach SessionStart; the delivery loop's mod health check (CONNECT_GRACE 15 s) ran before any session was bound and overwrote status_context 'start' with 'tui:mod-unavailable'. SessionStart's is_fresh_claude_process_placeholder accepts listening only with start/ready_observed/launch_blocked_cleared, so it logged sessionstart.lineage_unknown and did not bind; every later hook was rejected as historical_session, the mod never connected, and messages stayed queued (launch_blocked). Fix: claude_mod::report_health reports nothing while the row has no session bound.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 report_health leaves a listening row without a session_id unchanged (context stays 'start')
- [x] #2 A unit test covers the unbound row case
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
claude_mod::report_health returns early while the row has no session_id, so a slow Claude start no longer replaces the launch 'start' context and SessionStart binds. Verified: new unit test health_waits_for_the_session_bind (red before, green after), cargo test --bin hcom, cli_smoke lifecycle + claude filters, fmt, clippy.
<!-- SECTION:FINAL_SUMMARY:END -->

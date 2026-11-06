---
id: TASK-95
title: 'Dashboard: handoff relaunch detaches the agent from its spawn parent'
status: Done
assignee:
  - '@hcom_dev__idea'
created_date: '2026-11-06T22:33:00Z'
updated_date: '2026-11-06T22:49:00Z'
labels: []
dependencies: []
priority: medium
type: bug
ordinal: 143000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
hcom handoff relaunches the agent detached (hcom agent ... --restart, without the caller's identity), so its life events are handoff, stopped, created, batch_launched by=user. hcom-dashboard Hierarchy takes the parent from the last batch_launched since created, so a handed-off agent becomes a root instead of staying under the agent that launched it. User decision: hcom's launcher semantics stay unchanged (ready notification, reply suffix, authority line); a handoff is presented like an internal compact, not a new launch, so the fix lives only in contrib/hcom-dashboard.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 After a handoff event, the next created and batch_launched of that instance keep its previous launcher in the dashboard
- [x] #2 A later launch of the instance without a preceding handoff determines the launcher again
- [x] #3 Regression test in contrib/hcom-dashboard covers a handoff relaunch of an agent launched by another agent
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
User decision: hcom launcher semantics unchanged; a handoff is shown like a compact. compute_event_stats marks an instance on its handoff event and keeps its launcher across the next created and batch_launched (by=user from the detached relaunch); a later plain relaunch resets it. Verified with test_task_95_handoff_relaunch_keeps_the_launcher (fails without the fix) and dashboard fmt/clippy/test (17+185 ok).
<!-- SECTION:FINAL_SUMMARY:END -->

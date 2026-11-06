---
id: TASK-95
title: Handoff relaunch logs batch_launched by user
status: To Do
assignee: []
created_date: '2026-11-06T22:33:00Z'
labels: []
dependencies: []
priority: medium
type: bug
ordinal: 143000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
hcom handoff relaunch records batch_launched with by=user, so a handed-off agent becomes a root in the spawn tree (hcom-dashboard Hierarchy) instead of staying under the agent that launched it. Found while building the hcom-dashboard spawn tree (TASK-77.38).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 batch_launched of a handoff relaunch names the original launcher (or the agent that ran the handoff), not user
- [ ] #2 Regression test covers a handoff relaunch of an agent launched by another agent
<!-- AC:END -->

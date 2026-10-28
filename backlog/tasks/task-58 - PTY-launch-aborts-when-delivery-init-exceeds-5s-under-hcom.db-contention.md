---
id: TASK-58
title: PTY launch aborts when delivery init exceeds 5s under hcom.db contention
status: To Do
assignee: []
created_date: '2026-10-28 07:41'
labels: []
dependencies: []
priority: high
type: bug
ordinal: 59000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Observed: agent_coach started from hcom-gui (07:05:36Z 2026-10-28, Herdr pane) died with 'Error: PTY run failed / Caused by: delivery start timed out'. The delivery thread (src/pty/shared.rs, initialize_delivery_components: HcomDb::open, NotifyServer, register_inject_port) did not finish within the 5s recv_timeout; on Unix src/pty/mod.rs bails on DeliveryStart::Pending and kills the session. At the same time the agent's SessionStart hook took 4.98s and other agents' hooks took 1-3s (wdt_main pre Edit 2.8s, sphere_dev_ansible_proweb_505 pre Bash 3.2s), pointing to hcom.db lock contention. Plan agreed with the user: first find what holds hcom.db so long (root cause), and as a safeguard raise the delivery init timeout.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The source of the multi-second hcom.db waits during launches is identified with log/DB evidence and recorded in the task notes
- [ ] #2 The identified contention is fixed or reduced, verified by hook/delivery timings under a comparable concurrent load
- [ ] #3 The delivery init timeout is raised (e.g. 15-20s) so a launch survives a temporary DB stall, covered by a test
- [ ] #4 A launch whose delivery init still times out reports the reason in hcom logs and the launching command's error output
<!-- AC:END -->

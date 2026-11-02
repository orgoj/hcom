---
id: TASK-83
title: Send to a stopped roaming instance name autostarts it
status: To Do
assignee: []
created_date: '2026-11-02 20:00'
labels: []
dependencies: []
type: bug
ordinal: 111000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Both addresses of a roaming agent must stay usable: the role (@reviewer) and its project instance name (@reviewer_<project>_<hash>). When the instance is stopped and an agent sends to the instance name, hcom must recognize it as a roaming instance and autostart it the same way as a send to the role. Goal: as simple as possible for agents, no special handling on their side. Requested by the user.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 hcom send @<role>_<project>_<hash> to a stopped (or never started) roaming instance autostarts it and delivers the message, like hcom send @<role>
- [ ] #2 A send to the role name keeps working unchanged
- [ ] #3 Instance names of other projects are not started from a foreign project
- [ ] #4 cli_smoke regression test covers a send to the stopped instance name
<!-- AC:END -->

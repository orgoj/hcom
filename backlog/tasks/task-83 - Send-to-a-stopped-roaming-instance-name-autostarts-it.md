---
id: TASK-83
title: Send to a stopped roaming instance name autostarts it
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-11-02 20:00'
updated_date: '2026-11-02 20:12'
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
- [x] #1 A send to the role name keeps working unchanged
- [x] #2 cli_smoke regression test covers a send to the stopped instance name
- [x] #3 From the instance's own project, hcom send @<role>_<project>_<hash> to a stopped (or never started) roaming instance autostarts it and delivers the message, like hcom send @<role>
- [x] #4 From another project, a send to a stopped instance name keeps failing with 'No active agent' (user decision: own project only)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Probe (cli_smoke): send to the instance name from its own project autostarts it, also after hcom stop. From another project, after stop the row is gone and the send fails 'No active agent for @<instance>'; retarget_roaming_route (agent.rs) already starts a foreign project's instance at its own root, but only while its instances row exists. Stop events keep a pre-delete snapshot. AC3 conflicts with that existing retarget design; waiting for user decision.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Own-project sends to a stopped or never-started roaming instance name already autostart it; no code change. Added cli_smoke roaming_send_to_the_instance_name_autostarts_it_in_its_own_project covering never-started, role, after-stop and the foreign-project error. Verified: fmt, clippy, cli_smoke roaming (17 passed).
<!-- SECTION:FINAL_SUMMARY:END -->

---
id: TASK-50
title: 'hcom agent attach: focus the Herdr pane by pane_id, not by agent name'
status: To Do
assignee: []
created_date: '2026-10-27 06:42'
updated_date: '2026-10-27 06:55'
labels: []
dependencies: []
type: enhancement
ordinal: 51000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
focus_running (src/commands/agent.rs) runs 'herdr agent focus <name>', so attach depends on Herdr's agent naming. Once, attach from hcom-gui failed only for hcom_dev, which had just been relaunched via 'hcom handoff' (pane w3P:pK -> w3P:pM); minutes later the same command worked, so the cause was not reproduced. Suspected: Herdr's name -> pane mapping was stale or missing right after the relaunch. The live instance's launch_context already records pane_id. Find the Herdr command that focuses a pane by its id from Herdr's own help.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 hcom agent attach <name> for a live Herdr instance with launch_context.pane_id focuses that pane by pane_id
- [ ] #2 Without a pane_id in launch_context, attach keeps the current behavior (herdr agent focus <name>)
- [ ] #3 Non-Herdr instances and agents that are not running keep their current errors
- [ ] #4 Covered by a cli_smoke test, with help and docs updated if user-visible behavior changes
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Findings 2026-10-27: the GUI later reproduced it ('agent_not_found: agent target hcom_dev not found', exit 1) while the same command from a shell (cwd HOME, HCOM_* and HERDR_* unset) succeeded before and after. 60 s sampling of 'herdr agent list' (~1140 samples) always showed name hcom_dev for pane w3P:pM; samples with the pane missing would not show up, so a pane briefly absent from the agent list is not excluded. 'herdr pane focus' only moves to a neighbor (--direction); 'herdr agent focus <pane_id>' (e.g. w3P:pM) works, but it still needs Herdr to detect the pane as an agent. 'herdr agent explain' shows the detection state: run it at the moment of failure.
<!-- SECTION:NOTES:END -->

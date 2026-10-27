---
id: TASK-47
title: >-
  Note when a send or start resolves a roaming agent to its per-project instance
  name
status: To Do
assignee: []
created_date: '2026-10-26 20:34'
updated_date: '2026-10-27 05:25'
labels: []
dependencies: []
priority: medium
type: feature
ordinal: 48000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Goal: a sender who addresses a roaming agent by its catalog name (e.g. @sphere_dev) learns that the agent is roaming and which per-project instance the message reached, so it can address the right instance by its exact name.

Incident: cca_manager sent @sphere_dev from its own directory and, without noticing, started sphere_dev_cca_manager_369 instead of reaching sphere_dev_ansible_proweb_505 in the proweb clone.

User decision: routing stays as it is. The catalog name resolves to the instance for the sender's project root; when that instance runs, it receives the message, and when it is missing, send autostart starts it. Instances of the role in other projects are irrelevant and never cause an error. Only an informational note on stdout is added; stderr is avoided so agents do not read it as a failure.

In scope: a stdout note in hcom send feedback when a target given as a roaming catalog name (bare or tag-prefixed, @mention or thread member) resolves to <name>_<project>_<hash>; the same note on a send that autostarts the instance; a note when hcom agent <name> (and resume/show paths that materialize a roaming name) starts a roaming instance, naming the exact instance name for this directory.
Out of scope: errors, listing instances from other projects, any change to routing or autostart, changes to the global SYSTEM_PROMPT rule (still true).

Risk: the send code path is shared with upstream; keep the hook minimal in the fork's catalog routing (retarget_roaming_route / catalog_message_targets).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A running instance of the same role in another project neither changes routing nor produces an error
- [ ] #2 hcom send @<roaming name> from a project prints on stdout, next to the 'Sent to:' feedback, a note line (not worded as warning or error) that the agent is roaming and gives the resolved instance name <name>_<project>_<hash>; nothing new goes to stderr and the exit code and delivery are unchanged
- [ ] #3 The note also appears when the send autostarts the missing instance, and the instance still starts in the sender's project root
- [ ] #4 hcom agent <roaming name> prints the same kind of stdout note naming the instance for the current directory; the launch is unchanged
- [ ] #5 Built-in help (send and agent topics), README Roaming agents section and skills/hcom-agent-messaging describe the note; cli_smoke tests cover every case above
- [ ] #6 hcom send @<exact instance name> and sends to non-roaming catalog agents print no such note
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Find where the alias->instance resolution is known in send (catalog_starts / retarget_roaming_route) and in launch_named_with_roaming.
2. Add one stdout note line per resolved roaming alias to the send feedback and to the agent launch output.
3. cli_smoke cases: running instance, autostart, exact name, non-roaming, hcom agent start, stderr stays empty.
4. Docs/help + help tests.
<!-- SECTION:PLAN:END -->

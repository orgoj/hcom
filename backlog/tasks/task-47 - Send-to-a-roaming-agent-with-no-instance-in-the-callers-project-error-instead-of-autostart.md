---
id: TASK-47
title: >-
  Warn when a send or start resolves a roaming agent to its per-project instance
  name
status: To Do
assignee: []
created_date: '2026-10-26 20:34'
updated_date: '2026-10-27 05:20'
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

User decision: routing stays as it is. The catalog name resolves to the instance for the sender's project root; when that instance runs, it receives the message, and when it is missing, send autostart starts it. Instances of the role in other projects are irrelevant and never cause an error. Only a warning is added.

In scope: a warning on stderr of hcom send when a target given as a roaming catalog name (bare or tag-prefixed, @mention or thread member) resolves to <name>_<project>_<hash>; the same warning on a send that autostarts the instance; a warning when hcom agent <name> (and resume/show paths that materialize a roaming name) starts a roaming instance, naming the exact instance name for this directory.
Out of scope: errors, listing instances from other projects, any change to routing or autostart, changes to the global SYSTEM_PROMPT rule (still true).

Risk: the send code path is shared with upstream; keep the hook minimal in the fork's catalog routing (retarget_roaming_route / catalog_message_targets).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 hcom send @<roaming name> from a project prints on stderr a warning that names the agent as roaming and gives the resolved instance name <name>_<project>_<hash>; the message is delivered exactly as before
- [ ] #2 The warning also appears when the send autostarts the missing instance, and the instance still starts in the sender's project root
- [ ] #3 hcom send @<exact instance name> and sends to non-roaming catalog agents print no such warning
- [ ] #4 hcom agent <roaming name> prints a warning that the agent is roaming and names the instance for the current directory; the launch is unchanged
- [ ] #5 A running instance of the same role in another project neither changes routing nor produces an error
- [ ] #6 Built-in help (send and agent topics), README Roaming agents section and skills/hcom-agent-messaging describe the warning; cli_smoke tests cover every case above
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Find where the alias->instance resolution is known in send (catalog_starts / retarget_roaming_route) and in launch_named_with_roaming.
2. Emit one stderr warning per resolved roaming alias.
3. cli_smoke cases: running instance, autostart, exact name, non-roaming, hcom agent start.
4. Docs/help + help tests.
<!-- SECTION:PLAN:END -->

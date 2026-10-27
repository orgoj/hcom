---
id: TASK-47
title: >-
  Note when a send or start resolves a roaming agent to its per-project instance
  name
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-10-26 20:34'
updated_date: '2026-10-27 05:48'
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

In scope: a stdout note in hcom send feedback when a target given as a roaming catalog name (bare or tag-prefixed @mention or explicit target; thread-only sends address stored instance names, never an alias, so they get no note) resolves to <name>_<project>_<hash>; the same note on a send that autostarts the instance; a note when hcom agent <name> (including --resume and other flags on the same launch path) starts a roaming instance, naming the exact instance name for this directory.
Out of scope: errors, hcom agent show (already prints the instance), thread-only sends, listing instances from other projects, any change to routing or autostart, changes to the global SYSTEM_PROMPT rule (still true).

Risk: the send code path is shared with upstream; keep the hook minimal in the fork's catalog routing (retarget_roaming_route / catalog_message_targets).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A running instance of the same role in another project neither changes routing nor produces an error
- [x] #2 hcom send @<roaming name> from a project prints on stdout, next to the 'Sent to:' feedback, a note line (not worded as warning or error) that the agent is roaming and gives the resolved instance name <name>_<project>_<hash>; nothing new goes to stderr and the exit code and delivery are unchanged
- [x] #3 The note also appears when the send autostarts the missing instance, and the instance still starts in the sender's project root
- [x] #4 hcom agent <roaming name> prints the same kind of stdout note naming the instance for the current directory; the launch is unchanged
- [x] #5 Built-in help (send and agent topics), README Roaming agents section and skills/hcom-agent-messaging describe the note; cli_smoke tests cover every case above
- [x] #6 hcom send @<exact instance name> and sends to non-roaming catalog agents print no such note
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Record alias->instance pairs where resolve_delivery maps a roaming catalog alias (route.alias != route.instance) and add a stdout Note line per pair to the send feedback.
2. Print a stdout Note in launch_named_with_roaming when hcom agent <roaming name> materializes the instance itself (send autostart passes it and stays silent there).
3. cli_smoke: running instance, autostart while another project runs one (incident), exact name and non-roaming (no note), hcom agent and --resume; stderr carries no note.
4. Help topics, top-level overview, README, skill, help test.
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Send to a roaming catalog alias appends a stdout 'Note: @<alias> is a roaming agent; in this project it is <instance> (use that name).' line, taken from the actual delivery (SendOutcome); hcom agent <roaming> prints a similar note when it materializes the instance itself. Routing, autostart, exit code and stderr unchanged. Verified with cli_smoke roaming_send_autostarts_own_project_instance_while_another_project_runs_one (incident), roaming_send_from_same_basename_roots_addresses_distinct_instances, send_by_exact_or_non_roaming_name_prints_no_roaming_note, agent_launch_of_a_roaming_agent_notes_the_project_instance, unit send_outcome_reports_roaming_aliases_from_the_delivery_it_sent, help_documents_the_roaming_note_line; fmt, clippy, cargo test --bin hcom, cli_smoke roaming/send/agent/catalog/autostart/help green. Reviewed by agent_review.
<!-- SECTION:FINAL_SUMMARY:END -->

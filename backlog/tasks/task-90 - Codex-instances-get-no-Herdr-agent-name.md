---
id: TASK-90
title: Codex instances get no Herdr agent name
status: Idea
assignee: []
created_date: '2026-11-04 08:48'
updated_date: '2026-11-04 08:52'
labels: []
dependencies: []
type: bug
ordinal: 127000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Reported by herdr_dev (user request): `herdr agent list` shows `name` for hcom Claude panes (e.g. contimex_main, bin, agent_coach) but none for Codex panes w6G:p3 (agent_review_ansible_contimex_659) and w6G:p4 (agent_security_ansible_contimex_659). Their pane label is set (◉ <name> [codex]) and `hcom list` maps their pane_id correctly. Herdr agent names are set via the agent.rename API. Impact: plugins sorting by agent name (herdr-sort) put them last; herdr-sort now falls back to the pane label. Suggested direction: find why the agent.rename call is not made or not applied for Codex launches. Needs analysis before implementation.
<!-- SECTION:DESCRIPTION:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Cause found in ~/.hcom/.tmp/logs/hcom.log: host_label op_rejected every 30 s for agent_security_ansible_contimex_659 / agent_review_ansible_contimex_659: 'herdr: agent name must start with a lowercase letter and contain only lowercase letters, digits, '-' or '_' (1-32 characters)'. Both names are 35 and 33 characters; Claude names that work are shorter. Not Codex-specific: any roaming instance name over 32 characters fails. Open: how to map long names (truncate, hash suffix) and whether to stop retrying a rejected rename.
<!-- SECTION:NOTES:END -->

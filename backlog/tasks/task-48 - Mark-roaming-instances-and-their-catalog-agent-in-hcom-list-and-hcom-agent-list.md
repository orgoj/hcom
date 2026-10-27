---
id: TASK-48
title: >-
  Mark roaming instances and their catalog agent in hcom list and hcom agent
  list
status: To Do
assignee: []
created_date: '2026-10-26 21:02'
updated_date: '2026-10-27 05:20'
labels: []
dependencies: []
priority: medium
type: feature
ordinal: 49000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Goal: human listings show which running instance is roaming and which catalog agent it belongs to.

Findings: hcom agent list --json has roaming: true; plain hcom agent list marks roaming only as <project> in the DIR column, and its STATUS is looked up by catalog name, so a roaming entry always shows '○ -' even while instances run. hcom list shows no link at all (e.g. sphere_dev_ansible_proweb_505 has no visible relation to sphere_dev). The data exists: instance launch_context.agent_name plus the catalog's roaming flag; nothing new needs to be stored.

User decisions: hcom list shows a badge after the name like the headless/remote badges, e.g. 'sphere_dev_ansible_proweb_505 ↻sphere_dev'. hcom agent list shows the count and names of the running instances in the STATUS of the roaming entry, e.g. '● 2 roaming: sphere_dev_ansible_proweb_505, sphere_dev_cca_manager_369'.

In scope: plain hcom list badge, hcom agent list STATUS for roaming entries, an instances field in hcom agent list --json.
Out of scope: hcom-gui (TASK-49), badges for non-roaming catalog agents started with --as, changes to hcom list --json.
Risk: hcom list rendering is upstream code; keep the fork touch to a minimal hook that computes the badge.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 hcom list shows '↻<catalog name>' after the name of every running instance whose launch_context.agent_name is a roaming catalog agent and whose name differs from it
- [ ] #2 hcom list shows no such badge for non-roaming instances, including catalog agents started with --as
- [ ] #3 hcom agent list shows in the STATUS of a roaming entry '● <N> roaming: <instance names>' for its running instances, and '○ -' when none runs
- [ ] #4 hcom agent list --json gives each roaming entry an instances array with the names of its running instances
- [ ] #5 Built-in help, README and skills/hcom-agent-messaging describe both markers; cli_smoke tests cover each case above
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Helper: map running instance -> roaming catalog name (launch_context.agent_name + catalog roaming).
2. Badge in hcom list name_part via minimal hook.
3. agent list STATUS + JSON instances.
4. cli_smoke cases, docs/help.
<!-- SECTION:PLAN:END -->

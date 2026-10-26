---
id: TASK-48
title: >-
  Mark roaming instances and their catalog agent in hcom list and hcom agent
  list
status: Idea
assignee: []
created_date: '2026-10-26 21:02'
labels: []
dependencies: []
priority: medium
type: feature
ordinal: 49000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Request from the user (via agent_coach). Today only `hcom agent list --json` carries `roaming: true`. The plain `hcom agent list` shows it only in the description text, and `hcom list` not at all: e.g. sphere_dev_ansible_proweb_505 shows no link to its catalog agent sphere_dev. Both human listings should show that an instance is roaming and which catalog agent it belongs to.

Needs analysis before implementation.
<!-- SECTION:DESCRIPTION:END -->

---
id: TASK-123
title: Herdr rejects hcom agent names longer than 32 chars
status: Idea
assignee: []
created_date: '2026-11-19T09:45:00Z'
labels: []
dependencies: []
type: bug
ordinal: 234000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
agent_architect_killbottleneck_228 (34 chars): host_label logs op_rejected several times per second ('herdr: agent name must start with a lowercase letter and contain only lowercase letters, digits, - or _ (1-32 characters)'); the Herdr label is never set, attach by name cannot work and hcom.log floods. Suggested direction: retry the label at most once per name, and/or shorten the Herdr label. Needs analysis before implementation.
<!-- SECTION:DESCRIPTION:END -->

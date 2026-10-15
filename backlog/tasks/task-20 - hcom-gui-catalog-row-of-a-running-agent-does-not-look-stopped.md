---
id: TASK-20
title: 'hcom-gui: catalog row of a running agent does not look stopped'
status: To Do
assignee: []
created_date: '2026-10-15 13:34'
labels: []
dependencies: []
type: bug
ordinal: 19000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
When an agent runs only under another instance name (--as x), its catalog row agent| stays and is greyed as stopped, although the agent runs. After Start with --as, the old catalog row stays selected instead of the new instance.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A catalog row whose agent has running instances is not styled as stopped and its Status names the running instances
- [ ] #2 After Start with --as <name>, the new instance row is selected once it appears
- [ ] #3 Covered by contrib/test-hcom-gui
<!-- AC:END -->

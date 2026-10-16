---
id: TASK-27
title: 'hcom-gui: Where column shows the terminal type'
status: To Do
assignee: []
created_date: '2026-10-16 04:45'
updated_date: '2026-10-16 04:53'
labels: []
dependencies: []
type: feature
ordinal: 26000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The Where column shows only the Herdr workspace/tab (TASK-10), so it does not tell which terminal an agent runs in. Prefix it with the terminal from launch_context.terminal_preset_effective, for all terminals, not only Herdr.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Where shows '<terminal>: <workspace>/<tab>' for Herdr instances, e.g. 'herdr: cc-hcom/hcom_dev'
- [ ] #2 Instances in other terminals show the terminal name alone (e.g. 'kitty'); instances without terminal_preset_effective show an empty value
- [ ] #3 contrib/test-hcom-gui covers the Herdr, other-terminal and empty cases
<!-- AC:END -->

---
id: TASK-77.68
title: >-
  Dashboard: po startu agenta pod jiným jménem (--as) se vybere řádek nové
  instance
status: In Progress
assignee:
  - '@hcom_dev__idea'
created_date: '2026-11-07T13:22:00Z'
updated_date: '2026-11-07T13:30:00Z'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: high
type: bug
ordinal: 163000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Uživatel: když startuji agenta pod jiným jménem, nemám označený jeho řádek. hcom-gui (TASK-16): po Start s --as se nová instance vybere, jakmile se objeví, pokud mezitím uživatel nevybral jiný řádek; po Start/Restart zůstává výběr na tom agentovi.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Po Start s --as se po objevení nové instance vybere její řádek (tabulka i graf), pokud uživatel mezitím nevybral jiný
- [ ] #2 Start bez --as vybere instanci agenta stejně
- [ ] #3 Headless test přes Start dialog (cesta, kterou uživatel používá)
<!-- AC:END -->

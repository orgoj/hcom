---
id: TASK-77.68
title: >-
  Dashboard: po startu agenta pod jiným jménem (--as) se vybere řádek nové
  instance
status: Done
assignee:
  - '@hcom_dev__idea'
created_date: '2026-11-07T13:22:00Z'
updated_date: '2026-11-07T13:34:00Z'
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
- [x] #1 Po Start s --as se po objevení nové instance vybere její řádek (tabulka i graf), pokud uživatel mezitím nevybral jiný
- [x] #2 Start bez --as vybere instanci agenta stejně
- [x] #3 Headless test přes Start dialog (cesta, kterou uživatel používá)
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Start dialog OK records select_next (instance named by --as, else the agent, plus the selection at launch); a table reload that shows the instance selects its row and node (select_table_row) unless the selection changed meanwhile; a failed command clears it. Verified through the Start dialog by test_task_77_68_start_selects_the_started_instance (fails without the fix); agent_review approved.
<!-- SECTION:FINAL_SUMMARY:END -->

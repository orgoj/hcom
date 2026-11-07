---
id: TASK-77.63
title: 'Dashboard: dialog pro výběr a pořadí sloupců tabulky'
status: In Progress
assignee:
  - '@hcom_dev__idea'
created_date: '2026-11-07T12:00:00Z'
updated_date: '2026-11-07T13:33:00Z'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: medium
type: feature
ordinal: 158000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Jako Columns… v hcom-gui: za běhu zvolit viditelné sloupce a jejich pořadí; volba se ukládá do dashboard-state.json a má přednost před columns v dashboard.json. Teď jdou sloupce měnit jen v konfiguraci.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Dialog zobrazí všechny sloupce (včetně status:<name>) se zaškrtnutím viditelnosti
- [ ] #2 Pořadí sloupců jde v dialogu měnit (i klávesnicí)
- [ ] #3 Aspoň jeden sloupec zůstane viditelný
- [ ] #4 Volba se uloží do dashboard-state.json a přežije restart; Reset ji zahodí
- [ ] #5 Headless test ověří uložení a pořadí
<!-- AC:END -->

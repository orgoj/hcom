---
id: TASK-77.71
title: 'Dashboard: pamatovat maximalizované okno a velikost okna'
status: In Progress
assignee:
  - '@hcom_dev__idea'
created_date: '2026-11-07T14:58:00Z'
updated_date: '2026-11-07T15:00:00Z'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: medium
type: feature
ordinal: 166000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Uživatel 2026-11-07: po každém spuštění musí okno maximalizovat. Dashboard má jen pevné with_inner_size 1200x800 (src/main.rs); pole DashboardState.geometry se nepoužívá. Stav se ukládá do ~/.hcom/dashboard-state.json.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Maximalizované okno se po zavření a novém spuštění otevře maximalizované
- [ ] #2 Nemaximalizované okno se otevře v naposledy použité velikosti (pozici okna řídí správce oken)
- [ ] #3 Bez uloženého stavu je výchozí velikost 1200x800 jako dosud
- [ ] #4 Test pokrývá uložení stavu okna a jeho použití při startu
<!-- AC:END -->

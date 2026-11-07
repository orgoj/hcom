---
id: TASK-77.71
title: 'Dashboard: pamatovat maximalizované okno a velikost okna'
status: Done
assignee:
  - '@hcom_dev__idea'
created_date: '2026-11-07T14:58:00Z'
updated_date: '2026-11-07T15:09:00Z'
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
- [x] #1 Maximalizované okno se po zavření a novém spuštění otevře maximalizované
- [x] #2 Nemaximalizované okno se otevře v naposledy použité velikosti (pozici okna řídí správce oken)
- [x] #3 Bez uloženého stavu je výchozí velikost 1200x800 jako dosud
- [x] #4 Test pokrývá uložení stavu okna a jeho použití při startu
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Stav okna (window_maximized, window_size) v dashboard-state.json; main.rs otevře okno s with_maximized a uloženou velikostí, app ho zapisuje z viewport info. Testy test_task_77_71_*; review bez nálezů.
<!-- SECTION:FINAL_SUMMARY:END -->

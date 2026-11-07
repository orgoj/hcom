---
id: TASK-77.86
title: 'Dashboard: maximalizace se při startu neobnoví'
status: To Do
assignee: []
created_date: '2026-11-07T15:58:00Z'
updated_date: '2026-11-07T15:58:00Z'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: low
type: bug
ordinal: 181000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Vizuální kontrola 2026-11-07 (P5, navazuje na TASK-77.71). V gui-sandboxu: maximalizace, 5 s, zavření X → dashboard-state.json má window_maximized=true (ukládání funguje). Po spuštění se okno otevře 1200x800 nemaximalizované (v4-relaunch.png) a první snímek uloží false. main.rs předává with_maximized(maximized). Příčina neznámá: buď WM sandboxu ignoruje počáteční maximalizaci, nebo egui/winit. Nejdřív ověřit na desktopu uživatele; jen sandbox → uzavřít jako rejected s nálezem.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Na desktopu uživatele se po maximalizaci, zavření a novém spuštění okno otevře maximalizované
- [ ] #2 První snímek po startu nepřepíše uložené window_maximized=true dřív, než WM maximalizaci nahlásí
<!-- AC:END -->

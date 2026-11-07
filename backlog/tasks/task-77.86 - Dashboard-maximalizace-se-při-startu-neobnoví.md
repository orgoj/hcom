---
id: TASK-77.86
title: 'Dashboard: maximalizace se při startu neobnoví'
status: In Progress
assignee:
  - '@hcom_dev__idea'
created_date: '2026-11-07T15:58:00Z'
updated_date: '2026-11-07T17:16:00Z'
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
- [x] #2 První snímek po startu nepřepíše uložené window_maximized=true dřív, než WM maximalizaci nahlásí
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
User checks AC #1 on the desktop; then --check-ac 1, final summary, Done.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
User 2026-11-07: on the user's desktop (X11, LXQt) the window does not reopen maximized either, so not sandbox-only. Fix: pending_maximize asks once with ViewportCommand::Maximized(true) on the first frame and records no window state until the viewport reports maximized or 2 s (egui time) pass. AC #1 needs the user's check after deploy.

agent_review OK (tmp/knowledge/task-77.86-review.md). Committed and deployed; AC #1 waits for the user's desktop check (maximize, close, relaunch).
<!-- SECTION:NOTES:END -->

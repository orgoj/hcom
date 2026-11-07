---
id: TASK-77.96
title: Shift+klik na uzel přepíná pin
status: In Progress
assignee:
  - '@hcom_dev__idea'
created_date: '2026-11-07T22:20:00Z'
updated_date: '2026-11-07T22:23:00Z'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: medium
type: feature
ordinal: 192000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Požadavek uživatele: Shift+klik na uzel grafu přepne pin (Pin Position / Unpin Position, jako kontextové menu z TASK-77.17). Tažení se nemění: připnutý uzel po tažení zůstává připnutý, nepřipnutý nepřipnutý.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Shift+klik na nepřipnutý uzel ho připne na aktuální pozici a uloží pin (jako Pin Position)
- [x] #2 Shift+klik na připnutý uzel pin zruší (jako Unpin Position)
- [x] #3 Tažení uzlu (i se Shiftem) pin nemění
- [x] #4 README a nápověda kontextového menu uvádějí Shift+klik
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
CanvasAction::NodeShiftClicked from a Shift click (also each click of a fast Shift double click) calls toggle_pin; selection unchanged; drag untouched. Test test_shift_click_toggles_pin_and_shift_drag_keeps_it; dashboard fmt, clippy, 244 tests green. Small change, no review. Done after the user's check.
<!-- SECTION:NOTES:END -->

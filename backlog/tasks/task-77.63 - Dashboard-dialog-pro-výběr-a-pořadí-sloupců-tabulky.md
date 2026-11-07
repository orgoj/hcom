---
id: TASK-77.63
title: 'Dashboard: dialog pro výběr a pořadí sloupců tabulky'
status: Done
assignee:
  - '@hcom_dev__idea'
created_date: '2026-11-07T12:00:00Z'
updated_date: '2026-11-07T13:37:00Z'
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
- [x] #1 Dialog zobrazí všechny sloupce (včetně status:<name>) se zaškrtnutím viditelnosti
- [x] #2 Pořadí sloupců jde v dialogu měnit (i klávesnicí)
- [x] #3 Aspoň jeden sloupec zůstane viditelný
- [x] #4 Volba se uloží do dashboard-state.json a přežije restart; Reset ji zahodí
- [x] #5 Headless test ověří uložení a pořadí
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Columns… button at the end of the action toolbar opens a modal listing column_choices (shown columns in order, then ALL_COLUMNS and status:<name> from report fields and rows); click/Up/Down pick, Space/box/double-click toggle, Ctrl+Up/Down or Up/Down buttons move, Enter/OK apply only with a shown column, Escape/Cancel close, Reset sets default_columns; the choice goes to state.columns in dashboard-state.json. Verified by test_task_77_63_columns_dialog_chooses_and_orders_columns; agent_review approved.
<!-- SECTION:FINAL_SUMMARY:END -->

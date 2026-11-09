---
id: TASK-77.100
title: Výběr tabulky po zmizení řádku drží jeho index
status: In Progress
assignee:
  - '@hcom_dev'
created_date: '2026-11-09T19:16:00Z'
updated_date: '2026-11-09T19:28:00Z'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: medium
type: feature
ordinal: 197000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Požadavek uživatele: výběr se drží jména (instance, jinak agent); když vybraný řádek z tabulky zmizí (např. po Kill), vybere se řádek na stejném indexu v zobrazeném pořadí (poslední, když je tabulka kratší), jako contrib/hcom-gui (refresh: children[min(index, len-1)]). Po Startu dál platí výběr spuštěné instance (select_next, TASK-77.68). Uživatel tak může opakovaně mačkat Del na dalším řádku.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Když vybraný řádek v tabulce zmizí, vybere se řádek na jeho indexu v zobrazeném pořadí, nebo poslední řádek
- [x] #2 Během čekání na spuštěnou instanci (select_next) se výběr nenahrazuje
- [x] #3 Výběr uzlu grafu, který v tabulce nebyl, se nenahrazuje
- [x] #4 README popisuje chování
- [x] #5 Je-li na indexu zastavený řádek téhož katalogového agenta, jehož instance zmizela, vybere se následující řádek (u posledního předchozí)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
render_table: table_selected_at (id, index) of the last frame; a vanished selected row selects rows[min(index, last)] unless select_next is pending or the id was never shown. Test test_task_77_100_vanished_selected_row_hands_selection_to_its_index; dashboard fmt, clippy, 252 tests green. Small change, no review. Done after the user's check.

User check: after Kill of a catalog agent its stopped row took the index and got selected; wrong, the next row is expected.

Fix: table_selected_at carries the row's catalog agent; a stopped row of that agent at the index is skipped for the next row (previous at the end). Test extended; dashboard fmt, clippy, 252 tests green (budget test flaked once, passed on rerun, known per NOTES). Small change, no review.
<!-- SECTION:NOTES:END -->

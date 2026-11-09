---
id: TASK-77.103
title: Samostatné filtry pro graf a tabulku v jejich záhlaví
status: Done
assignee: []
created_date: '2026-11-09T21:15:00Z'
updated_date: '2026-11-09T21:44:00Z'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: medium
type: feature
ordinal: 200000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Požadavek uživatele: graf a tabulka mají každý svůj FILTER, umístěný v záhlaví grafu a v záhlaví tabulky, ukládaný (dashboard-state.json). Na tabulku se nemá aplikovat WINDOW (rozsah okna zpráv): dnes Running only používá TableRow::ran_in(window_bounds) (table_data.rs), takže tabulka ukazuje i agenty, kteří v okně běželi, s jejich aktuálním stavem. Dnešní stav: jedna skupina FILTER v 2. řádku top baru (search, Running only, Blocked only, Hide disabled, Hide ad-hoc, Filter table); filtered_rows v app.rs (kolem ř. 4580) řídí graf i tabulku, checkbox Filter table (state.filter_table) přepíná tabulku na všechny agenty; Kill group Filtered (N) bere filtered_rows; / fokusuje search_query. Realizace v další session po hcom-handoff. Otevřené otázky viz Implementation Notes. Needs analysis before implementation.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Graf a tabulka mají každý vlastní filtr (search, Running only, Blocked only, Hide disabled, Hide ad-hoc): graf v pruhu nad plátnem, tabulka v pruhu nad hlavičkou sloupců; skupina FILTER z 2. řádku top baru zmizí
- [x] #2 Oba filtry se ukládají v dashboard-state.json a po restartu platí
- [x] #3 Running only tabulky znamená běží teď, bez okna zpráv; graf dál používá okno
- [x] #4 Checkbox Filter table je zrušen; Kill group Filtered (N) bere agenty z filtru tabulky
- [x] #5 / fokusuje hledání tabulky
- [x] #6 Výběr agenta skrytého filtrem grafu zůstává (tabulka, detail), graf ho jen nezvýrazní
- [x] #7 README popisuje oba filtry
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. DashboardState: graph_filter + table_filter (search, running_only, blocked_only, hide_disabled, hide_adhoc); drop filter_table. 2. app.rs ~4580: filtered_rows for the graph (with window), table_rows from the table filter (is_running). 3. Strips: graph above canvas, table above the grid header; remove FILTER from top bar row 2. 4. Kill group Filtered from table_rows; / to table search. 5. Tests (headless, state roundtrip, running-now table), README.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Open questions for the user: (1) table without WINDOW: Running only = running now? (2) both filters full set incl. own search field, or search shared? (3) Filter table checkbox removed? (4) Kill group Filtered (N) uses the table filter? (5) / focuses which search? (6) graph header: new strip above the canvas, table header: strip above the table header row, placement to confirm. (7) graph hides an agent selected in the table: keep selection?

User answers: (1) table Running only = running now, no WINDOW; (2) each panel its own search field; (3) Filter table checkbox removed; (4) Kill group Filtered (N) uses the table filter; (5) / focuses the table search; (6) graph filter strip above the canvas, table filter strip above the column header, FILTER group leaves top bar row 2; (7) a table selection hidden in the graph stays selected, the graph just does not highlight it.

Implemented: PanelFilter (search + 4 checkboxes) as state graph_filter / table_filter, search text persisted too; old flat fields and filter_table dropped without migration (no fallbacks before numbered releases). Each strip ends with its own Shown N / M; the top bar row 1 count moved into the strips. Alt+X clears the table search. Graph-only view has no table strip, so / does nothing there. Review: agent_review.

User checked in the app: OK.
<!-- SECTION:NOTES:END -->

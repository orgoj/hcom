---
id: TASK-77.103
title: Samostatné filtry pro graf a tabulku v jejich záhlaví
status: Idea
assignee: []
created_date: '2026-11-09T21:15:00Z'
updated_date: '2026-11-09T21:15:00Z'
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

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Open questions for the user: (1) table without WINDOW: Running only = running now? (2) both filters full set incl. own search field, or search shared? (3) Filter table checkbox removed? (4) Kill group Filtered (N) uses the table filter? (5) / focuses which search? (6) graph header: new strip above the canvas, table header: strip above the table header row, placement to confirm. (7) graph hides an agent selected in the table: keep selection?
<!-- SECTION:NOTES:END -->

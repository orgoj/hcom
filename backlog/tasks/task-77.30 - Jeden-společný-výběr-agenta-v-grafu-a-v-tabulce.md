---
id: TASK-77.30
title: Jeden společný výběr agenta v grafu a v tabulce
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-11-02 21:01'
updated_date: '2026-11-03 12:09'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: high
type: bug
ordinal: 114000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Uživatel: klik na uzel v grafu nevybere uzel. Graf a tabulka jsou rovnocenná zobrazení; výběr uzlu v grafu je totéž co výběr řádku v tabulce a naopak. Vždy existuje nejvýše jeden vybraný agent, stejný v obou zobrazeních a v detail panelu.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Klik na uzel v grafu vybere tohoto agenta: uzel je zvýrazněný, v tabulce je vybraný jeho řádek a detail panel ukazuje jeho data
- [x] #2 Výběr řádku v tabulce (klik i šipky) zvýrazní stejného agenta v grafu
- [x] #3 Výběr zůstane stejný při skrytí a znovuzobrazení tabulky nebo grafu (přepínání variant layoutu Both, Graph only, Table only)
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Add ViewMode enum (Both, GraphOnly, TableOnly) to GraphState. 2. Implement view switcher in toolbar and layout rendering. 3. Unify table row and graph node selection. 4. Single-click purely selects, double-click attaches. 5. Add regression tests.
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Sjednocen výběr agenta napříč grafem, tabulkou a detail panelem. Klik na uzel vybere agenta ve všech pohledech bez spuštění attach; double-click na uzel provede attach. Klik na hranu vybere vazbu a odznačí uzel; klik do prázdna zruší výběr. Implementován volič layoutu (Both/Graph/Table) včetně F4 a migrace stavu. Pokryto testy.
<!-- SECTION:FINAL_SUMMARY:END -->

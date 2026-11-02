---
id: TASK-77.30
title: Jeden společný výběr agenta v grafu a v tabulce
status: To Do
assignee: []
created_date: '2026-11-02 21:01'
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
- [ ] #1 Klik na uzel v grafu vybere tohoto agenta: uzel je zvýrazněný, v tabulce je vybraný jeho řádek a detail panel ukazuje jeho data
- [ ] #2 Výběr řádku v tabulce (klik i šipky) zvýrazní stejného agenta v grafu
- [ ] #3 Výběr zůstane stejný při skrytí a znovuzobrazení tabulky nebo grafu
<!-- AC:END -->

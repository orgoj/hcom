---
id: TASK-77.16
title: Splity drží nastavené proporce a přenášejí se mezi displeji
status: To Do
assignee: []
created_date: '2026-11-01 19:39'
updated_date: '2026-11-01 19:40'
labels: []
dependencies:
  - TASK-77.15
parent_task_id: TASK-77
priority: high
type: feature
ordinal: 94000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Rozhodnutí uživatele: pozice splitů je fixní – jak ji uživatel posune, tak zůstane, a při přepnutí na jiný displej (jiná velikost okna) se drží proporce. Týká se splitů nového layoutu (graf | záložky, horní část | tabulka). Dnes se část ukládá v pixelech (sash_pos, side_panel_width).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Každý split se ukládá do graph-state.json jako podíl velikosti okna, ne v pixelech
- [ ] #2 Po posunu splitu zůstane jeho pozice beze změny, dokud ho uživatel znovu neposune (žádné automatické přepočítání obsahem)
- [ ] #3 Po restartu na okně jiné velikosti mají splity stejné proporce jako před restartem
- [ ] #4 Neplatné uložené hodnoty (mimo rozsah, ne-konečné) se ořežou na rozumný rozsah bez pádu
<!-- AC:END -->

---
id: TASK-77.16
title: Splity drží nastavené proporce a přenášejí se mezi displeji
status: Done
assignee:
  - '@hcom_dev__idea'
created_date: '2026-11-01 19:39'
updated_date: '2026-11-01 20:06'
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
- [x] #1 Každý split se ukládá do graph-state.json jako podíl velikosti okna, ne v pixelech
- [x] #2 Po posunu splitu zůstane jeho pozice beze změny, dokud ho uživatel znovu neposune (žádné automatické přepočítání obsahem)
- [x] #3 Po restartu na okně jiné velikosti mají splity stejné proporce jako před restartem
- [x] #4 Neplatné uložené hodnoty (mimo rozsah, ne-konečné) se ořežou na rozumný rozsah bez pádu
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Review agent_review: plan OK (notes 1-7 applied), diff: P1 legacy conversion not persisted (fixed: mark_view_dirty + regression test), P2 README wording (fixed); re-review passed. Approach: egui resizable panels replaced by exact_size panels plus own foreground split handles, because egui keeps its own size and grows panels to content.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Table height and detail panel width are stored as window fractions (table_height_frac, detail_width_frac); panels have an exact size from them, content never resizes them, only dragging the split handles (double-click resets) changes them. Legacy px (sash_pos, side_panel_width) are converted once a window size is known and saved. Invalid fractions are dropped or clamped to 5-95 %; tiny/zero windows do not panic. Verified: headless drag/content/window-size/legacy tests, fmt, clippy 0, 98 tests.
<!-- SECTION:FINAL_SUMMARY:END -->

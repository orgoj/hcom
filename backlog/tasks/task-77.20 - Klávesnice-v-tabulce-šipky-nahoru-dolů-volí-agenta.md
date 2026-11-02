---
id: TASK-77.20
title: 'Klávesnice v tabulce: šipky nahoru/dolů volí agenta'
status: Done
assignee:
  - '@hcom_dev__idea'
created_date: '2026-11-01 19:40'
updated_date: '2026-11-02 19:17'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: medium
type: feature
ordinal: 98000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Rozhodnutí uživatele: graf se ovládá jen myší; klávesnice jen v tabulce, šipky nahoru/dolů pro volbu agenta.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Šipka dolů/nahoru vybere další/předchozí řádek tabulky v aktuálním řazení a filtru
- [x] #2 Změna výběru šipkami se projeví v grafu a v panelu záložek stejně jako klik
- [x] #3 Vybraný řádek se při pohybu šipkami posune do viditelné části tabulky
- [x] #4 Šipky nemění výběr, když má fokus textové pole (filtr, inject)
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Arrow keys in render_table unless a text edit is focused; shared select_table_row with the click path; scroll_to_rect for the key-moved row; README; headless tests.
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Up/Down in render_table move the selection over the shown (filtered, sorted) rows, clamped at the ends, first/last row when nothing is selected; selection goes through select_table_row shared with the click; the key-moved row is scrolled into view; ignored while a text field has focus (Context::text_edit_focused). Verified with test_table_arrow_keys_select_rows_in_shown_order, fmt, clippy 0 warnings, cargo test 17+114 ok. Reviewed by agent_review.
<!-- SECTION:FINAL_SUMMARY:END -->

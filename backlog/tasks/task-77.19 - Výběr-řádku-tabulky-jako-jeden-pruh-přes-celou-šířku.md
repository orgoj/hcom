---
id: TASK-77.19
title: Výběr řádku tabulky jako jeden pruh přes celou šířku
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-11-01 19:40'
updated_date: '2026-11-02 18:21'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: high
type: feature
ordinal: 97000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
UX review F2/F13/S2, výtka uživatele: vybraný řádek se kreslí jako samostatné pilulky za textem každé buňky, prázdné buňky jako modré čtverečky. Má to být jeden souvislý pruh přes celý řádek; hover zvýrazní celý řádek; záhlaví lehčí; zkrácený text s …
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Vybraný řádek pokrývá jeden souvislý pruh přes celou šířku tabulky včetně prázdných buněk a mezer; žádná buňka nemá vlastní tvar zvýraznění
- [x] #2 Klik kamkoli do řádku (i do prázdné buňky) řádek vybere a uzel v grafu dostane prstenec výběru
- [x] #3 Hover zvýrazní celý řádek slabším pruhem než výběr
- [x] #4 Zkrácený text buňky končí … a plná hodnota je v tooltipu
- [x] #5 Záhlaví sloupců nejsou vizuálně těžší než data (žádné plné pilulky); řazení zůstává funkční
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Bez nové závislosti, v render_table (app.rs) zůstává egui::Grid:
1. Grid::with_row_color místo striped: řádek 0 (záhlaví) bez barvy; vybraný datový řádek selection.bg_fill; hover řádek selection.bg_fill.gamma_multiply(0.4); ostatní původní pruhování (faint_bg_color na lichých řádcích). Grid tak kreslí jeden pruh přes celou šířku mřížky včetně mezer a prázdných buněk.
2. Buňky jako egui::Label::new(text).truncate() (bez vlastního tvaru); Grid::max_col_width(300) — egui elipsuje … a ukazuje plnou hodnotu v tooltipu. Text vybraného řádku v barvě selection.stroke.
3. Klik a hover nad pruhem řádku: během kreslení sebrat y-rozsah buněk každého řádku, po Grid::show pruh = x-rozsah mřížky × y-rozsah (± spacing.y/2); hover = ui.rect_contains_pointer(pruh) (respektuje vrstvy nad tabulkou), klik = primary_clicked v pruhu -> stávající logika výběru (selected_row_id, canvas.selected_node, on_selection_changed). Hover index uložit do App pro barvu řádku v dalším snímku; pruhy uložit do pub pole pro test.
4. Záhlaví: Button::new(RichText strong).frame(false) — bez pilulky, řazení cycle_sort beze změny.
5. Headless testy (run_app_events): klik do prázdné buňky vybere řádek a selected_node; výstupní tvary obsahují jediný obdélník barvy výběru o šířce mřížky; hover dá slabší pruh; dlouhá hodnota je zkrácená (galley elided) a záhlaví nemá rámeček.
Mimo rozsah: egui_extras TableBuilder, změny sloupců/řazení/filtrů.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Plan deviation (approved in review): row click/hover via ui.interact on the last frame's row bands registered before the cells, so scroll bars and header buttons keep priority and labels keep their elision tooltips.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Table rows: Grid::with_row_color paints one full-width band for the selected row and a weaker hover band; cells are truncated labels (max 300 px, … with full-value tooltip); row click anywhere (empty cells included) selects the row and its graph node; headers are frameless buttons and still sort. Verified with test_table_row_selection_is_one_full_width_band, fmt, clippy, 107 tests; reviewed by agent_review.
<!-- SECTION:FINAL_SUMMARY:END -->

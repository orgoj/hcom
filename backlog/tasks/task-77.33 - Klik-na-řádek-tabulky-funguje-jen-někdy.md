---
id: TASK-77.33
title: Klik na řádek tabulky funguje jen někdy
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-11-03 11:30'
updated_date: '2026-11-03 12:32'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: high
type: bug
ordinal: 117000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Uživatel: klik na řádku tabulky funguje jen někdy. Kliknutí na libovolný řádek tabulky (včetně libovolné buňky) musí spolehlivě vybrat daného agenta a synchronizovat výběr s grafem i detail panelem.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Kliknutí na řádek tabulky spolehlivě a okamžitě vybere daného agenta
- [x] #2 Kliknutí na libovolnou buňku nebo volné místo řádku tabulky vyvolá výběr
- [x] #3 Výběr řádku v tabulce se spolehlivě promítne do grafu a detail panelu
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Popiskům buněk v tabulce bylo přidáno Sense::click(), což umožňuje spolehlivý výběr řádku kliknutím přímo na text v libovolném sloupci. Podkladové obdélníky table_row_bands zůstávají zachovány pro kliknutí do prázdného místa na konci řádku. Výběr řádku se spolehlivě promítne do grafu i detail panelu. Pokryto testem test_task_77_33_cell_text_and_empty_space_click_selects_row.
<!-- SECTION:FINAL_SUMMARY:END -->

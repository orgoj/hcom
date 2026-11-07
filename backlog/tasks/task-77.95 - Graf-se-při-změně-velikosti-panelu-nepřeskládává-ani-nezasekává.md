---
id: TASK-77.95
title: Graf se při změně velikosti panelu nepřeskládává ani nezasekává
status: In Progress
assignee:
  - '@hcom_dev__idea'
created_date: '2026-11-07T22:08:00Z'
updated_date: '2026-11-07T22:15:00Z'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: high
type: bug
ordinal: 191000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Tažení oddělovače panelu (tabulka, detail) zasekává UI na sekundy a graf se přeskládá. Příčina: v Hierarchy set_canvas_size (layout.rs) při každém přechodu velikosti plátna přes hranici slotu/vrstvy přepočítá tree_targets a spustí start_transition; planned_positions běží synchronně až 3000 kroků bez limitu snímku, pak 4 s přesun uzlů. Navíc auto_fit refituje zoom a pan každý snímek, dokud uživatel nesáhne na pohled, takže po resize graf poskočí. Rozhodnutí uživatele: rozložení určuje jen graf, nezávisí na okně; při změně velikosti se mění jen offset (střed grafu zůstává ve středu plátna, zoom beze změny); auto-fit neexistuje průběžně, jen jednou na startu a tlačítkem Fit.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Změna velikosti plátna nemění pozice uzlů, nespouští přechod ani přepočet rozložení v žádném režimu
- [x] #2 Při změně velikosti plátna zůstává zoom a bod grafu ve středu plátna zůstává ve středu
- [x] #3 Zalomení osamocených kořenů v Hierarchy závisí jen na grafu (pevná referenční velikost), ne na okně
- [x] #4 Automatické přizpůsobení pohledu proběhne jen jednou (start bez uloženého pohledu, uložený pohled bez viditelného uzlu); Fit funguje jako dosud
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Remove set_canvas_size; tree_targets uses fixed TREE_REFERENCE_SIZE cells. 2. auto_fit is one-shot. 3. Regression test test_canvas_resize_keeps_layout_zoom_and_centered_point. 4. Review by agent_review, commit, release build.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented (uncommitted, in review): set_canvas_size removed, tree_cells() from a fixed 1600x900 reference; auto_fit cleared after its fit. New test fails on the old sources (auto_fit assertion), passes now; dashboard fmt, clippy, all tests green. README Layout modes and Fit to content updated.

Review agent_review: clean. Done after the user's check of the live dashboard.
<!-- SECTION:NOTES:END -->

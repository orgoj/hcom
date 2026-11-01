---
id: TASK-77.18
title: 'Fit to content: po startu a tlačítkem Fit jsou vidět všechny zobrazené uzly'
status: Done
assignee:
  - '@hcom_dev__idea'
created_date: '2026-11-01 19:39'
updated_date: '2026-11-01 20:20'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: high
type: feature
ordinal: 96000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
UX review F1/S1: po startu a po Reset View je plátno prázdné, protože pohled centruje počátek světa, ne uzly. Reset View nahradit tlačítkem Fit (zoom a pan na bounding box zobrazených uzlů vč. popisků, okraj ~5 %, auto-fit max 100 %). Ovládání grafu jen myší (rozhodnutí uživatele), proto bez klávesové zkratky.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Bez uloženého pohledu jsou po startu všechny zobrazené uzly uvnitř plátna bez zásahu uživatele
- [x] #2 Tlačítko Fit nastaví pohled tak, že všechny zobrazené uzly i jejich popisky jsou uvnitř plátna
- [x] #3 Uložený pohled, ve kterém není vidět žádný uzel, se po startu automaticky přizpůsobí
- [x] #4 Když je některý zobrazený uzel mimo pohled, ukazuje značka na příslušném okraji plátna
- [x] #5 Auto-fit neběží v režimu Manual ani při stisknutém tlačítku myši
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Review agent_review: plan OK with P1 (Fit button caps at ZOOM_MAX, auto-fit at 100 %); diff P1 (explicit Fit must end auto-fit) fixed with regression test; re-review passed.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Reset View replaced by Fit (canvas::fit_view: largest zoom where nodes, rings and labels fit with 5 % margin; button up to 400 %, auto-fit up to 100 %). Auto-fit runs at start without a saved view or when the saved view shows no node, follows the layout until the user zooms, pans, drags or presses Fit; never in Manual or with a pressed button. Off-view nodes get status-colored edge markers (canvas::edge_marker). Verified: fmt, clippy 0 warnings, 104 tests incl. new fit/marker/app tests, each app test mutation-checked.
<!-- SECTION:FINAL_SUMMARY:END -->

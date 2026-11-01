---
id: TASK-77.18
title: 'Fit to content: po startu a tlačítkem Fit jsou vidět všechny zobrazené uzly'
status: To Do
assignee: []
created_date: '2026-11-01 19:39'
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
- [ ] #1 Bez uloženého pohledu jsou po startu všechny zobrazené uzly uvnitř plátna bez zásahu uživatele
- [ ] #2 Tlačítko Fit nastaví pohled tak, že všechny zobrazené uzly i jejich popisky jsou uvnitř plátna
- [ ] #3 Uložený pohled, ve kterém není vidět žádný uzel, se po startu automaticky přizpůsobí
- [ ] #4 Když je některý zobrazený uzel mimo pohled, ukazuje značka na příslušném okraji plátna
- [ ] #5 Auto-fit neběží v režimu Manual ani při stisknutém tlačítku myši
<!-- AC:END -->

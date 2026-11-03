---
id: TASK-77.32
title: Klik na uzel v grafu nesmí centrovat plátno ani hýbat s grafem
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-11-03 11:30'
updated_date: '2026-11-03 12:25'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: high
type: bug
ordinal: 116000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Uživatel: klik na uzel nesmí centrovat ten uzel a nijak hýbat s grafem. Výběr uzlu má pouze změnit výběr (highlight uzlu, výběr řádku v tabulce, detail panel), ale pozice plátna (pan) a zoom musí zůstat beze změny.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Klik na uzel v grafu nezmění pan (pozici plátna) ani zoom
- [x] #2 Klik na uzel nevyvolá automatické centrování plátna na vybraný uzel
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Klik na uzel v grafu již nepohybuje s uzlem ani necestuje plátnem. Přetahování uzlu se aktivuje až po překročení 3px prahu s uchováním kandidáta stisknutého uzlu, zatímco posun prázdného plátna je do té doby potlačen. auto_fit se vypíná ihned při stisknutí tlačítka myši nad plátnem před vyhodnocením fit_canvas, což zamezuje vycentrování při uvolnění tlačítka myši. Pokryto integračními testy.
<!-- SECTION:FINAL_SUMMARY:END -->

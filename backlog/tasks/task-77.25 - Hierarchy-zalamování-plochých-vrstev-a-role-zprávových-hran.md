---
id: TASK-77.25
title: 'Hierarchy: zalamování plochých vrstev a role zprávových hran'
status: Done
assignee:
  - '@hcom_dev__idea'
created_date: 2026-11-01 19:40
updated_date: '2026-11-06T21:14:00Z'
labels: []
dependencies:
  - TASK-77.18
parent_task_id: TASK-77
priority: medium
type: feature
ordinal: 103000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
UX review F7 (část S1): s málo spawn hranami je Hierarchy jedna dlouhá řada přes celou šířku, zprávové pružiny ji rozmazávají. Otázka Q5 (role zprávových hran) se rozhodne při specifikaci.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Kořeny bez potomků se při vrstvě širší než plátno zalamují do dalších řad pod stromy
- [x] #2 Hierarchy se po Fit vejde do plátna bez uzlů za okrajem
- [x] #3 Hrany zpráv v Hierarchy se kreslí jako v jiných režimech; čistý strom dá vypnutí vrstvy Msgs (TASK-77.45), rozhodnutí uživatele 2026-11-06
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Uživatel (2026-11-06): částečně vyřešeno TASK-77.38 (spawn hrany podle batch_launched.by), Hierarchy už není jedna řada kořenů. Zbývá ověřit na živých datech, zda kořeny bez potomků ještě přetékají šířku (AC #1, #2). AC #3 (zprávové hrany v Hierarchy) může pokrýt samostatné vypínání vrstev hran z TASK-77.45.

Uživatel (2026-11-06): AC #3 pokryje přepínač vrstvy Msgs z TASK-77.45.

Implementace: tree_targets bere velikost plátna při zoomu 1 v slotech a vrstvách; když je horní vrstva širší než plátno, kořeny bez potomků jdou do vystředěných řad pod stromy, délka řady (aspoň šířka stromů) je ta s největším zoomem Fit pro poměr stran plátna. První známá velikost plátna (start) přesune nepřipnuté uzly rovnou na cíle, jinak zůstaly ~1600 px od cíle (pomalé cestování, TASK-77.46). Pozdější resize: uzly cestují fyzikou (TASK-77.46). Skryté uzly filtrem dál zabírají sloty (beze změny). AC #3: hrany zpráv v Hierarchy beze změny, čistý strom dává přepínač Msgs z TASK-77.45. Review agent_review OK.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Hierarchy zalamuje osamělé kořeny do řad pod stromy podle velikosti plátna (layout.rs tree_targets/set_canvas_size, app.rs volání před step). Ověřeno testy test_tree_targets_wrap_lone_roots_below_trees_when_top_layer_is_too_wide a test_hierarchy_with_many_lone_roots_fits_canvas_above_min_zoom (60 osamělých kořenů: všechny uzly v plátně po Fit, zoom > 2× ZOOM_MIN); fmt, clippy 0 warnings, cargo test 17+179.
<!-- SECTION:FINAL_SUMMARY:END -->

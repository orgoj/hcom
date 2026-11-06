---
id: TASK-77.25
title: 'Hierarchy: zalamování plochých vrstev a role zprávových hran'
status: To Do
assignee: []
created_date: 2026-11-01 19:40
updated_date: '2026-11-06T20:57:00Z'
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
- [ ] #1 Kořeny bez potomků se při vrstvě širší než plátno zalamují do dalších řad pod stromy
- [ ] #2 Hierarchy se po Fit vejde do plátna bez uzlů za okrajem
- [ ] #3 Hrany zpráv v Hierarchy se kreslí jako v jiných režimech; čistý strom dá vypnutí vrstvy Msgs (TASK-77.45), rozhodnutí uživatele 2026-11-06
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Uživatel (2026-11-06): částečně vyřešeno TASK-77.38 (spawn hrany podle batch_launched.by), Hierarchy už není jedna řada kořenů. Zbývá ověřit na živých datech, zda kořeny bez potomků ještě přetékají šířku (AC #1, #2). AC #3 (zprávové hrany v Hierarchy) může pokrýt samostatné vypínání vrstev hran z TASK-77.45.

Uživatel (2026-11-06): AC #3 pokryje přepínač vrstvy Msgs z TASK-77.45.
<!-- SECTION:NOTES:END -->

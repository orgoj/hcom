---
id: TASK-77.17
title: 'Pozice uzlů jen u uživatelem označených (pin), ne automaticky při tažení'
status: To Do
assignee: []
created_date: '2026-11-01 19:39'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: medium
type: feature
ordinal: 95000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Rozhodnutí uživatele (nahrazuje S4 z UX review a mění chování z TASK-77.10): pozice se ukládá jen u uzlů, které si uživatel výslovně označí; tažení samo nic neukládá. Označený uzel drží svou pozici i v simulaci a po restartu.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Uzel lze myší označit a odznačit (např. v kontextovém menu uzlu); označení je na uzlu vizuálně patrné
- [ ] #2 Tažení neoznačeného uzlu neukládá jeho pozici do graph-state.json
- [ ] #3 Označený uzel se po restartu objeví na uložené pozici a simulace s ním nehýbe
- [ ] #4 Odznačení odstraní uloženou pozici; stávající node_positions z TASK-77.10 se převedou nebo zahodí bez pádu
<!-- AC:END -->

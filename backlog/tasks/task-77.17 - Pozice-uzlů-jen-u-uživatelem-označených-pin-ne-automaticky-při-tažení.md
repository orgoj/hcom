---
id: TASK-77.17
title: 'Pozice uzlů jen u uživatelem označených (pin), ne automaticky při tažení'
status: Done
assignee:
  - '@hcom_dev__idea'
created_date: '2026-11-01 19:39'
updated_date: '2026-11-02 19:13'
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
- [x] #1 Uzel lze myší označit a odznačit (např. v kontextovém menu uzlu); označení je na uzlu vizuálně patrné
- [x] #2 Tažení neoznačeného uzlu neukládá jeho pozici do graph-state.json
- [x] #3 Označený uzel se po restartu objeví na uložené pozici a simulace s ním nehýbe
- [x] #4 Odznačení odstraní uloženou pozici; stávající node_positions z TASK-77.10 se převedou nebo zahodí bez pádu
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
See review plan: drop legacy node_positions, new pinned_positions key; pin/unpin in node context menu; pinned nodes pinned in simulation in all modes; drag saves only pinned nodes; pin marker on node; README + tests.
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Pin/Unpin Position in the node context menu with a light dot marker; drags of unpinned nodes store nothing, drags of pinned nodes move the pin; pinned nodes start at their pin and are pinned in the simulation in every mode, also when the initial sync runs before the persisted view is applied; new graph-state key pinned_positions, legacy node_positions ignored and dropped on save. Verified with model_tests (pin layout in all modes, drag release, restart ordering, legacy key, accesskit-driven menu test), fmt, clippy 0 warnings, cargo test 17+113 ok. Reviewed by agent_review.
<!-- SECTION:FINAL_SUMMARY:END -->

---
id: TASK-77.61
title: >-
  Dashboard: vybraný uzel, jeho přímí sousedé a hrany zvýrazněné, ostatní
  ztlumené
status: Done
assignee:
  - '@hcom_dev__idea'
created_date: '2026-11-07T11:48:00Z'
updated_date: '2026-11-07T11:50:00Z'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: high
type: bug
ordinal: 156000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Uživatel: zvýraznění zvoleného uzlu a jeho hran není vidět (jen prstenec). Musí se odlišit vybraný uzel a jeho přímá spojení, např. sytostí barvy.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Při výběru uzlu zůstane vybraný uzel, jeho přímí sousedé a jeho hrany v plné barvě
- [x] #2 Ostatní uzly, hrany, odznaky a popisky se ztlumí; bez výběru se nic netlumí
- [x] #3 Test ověří, co patří do zvýraznění
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
With a selected node, the node, its direct neighbours (neighbors_undirected) and its edges keep full color; other edges (message, spawn, initiator) and their badges fade to 25 % (FOCUS_DIM), other nodes get a canvas-colored veil over body and rings and their labels fade. Helpers focus_nodes / dims_edge in canvas.rs, test test_task_77_61_selected_node_focus_dims_the_rest, README updated. Review: agent_review approved.
<!-- SECTION:FINAL_SUMMARY:END -->

---
id: TASK-77.60
title: >-
  Dashboard: uzly grafu nesmí překmitávat, po puštění plynulý přesun do spočtené
  cílové polohy
status: Done
assignee: []
created_date: '2026-11-07T11:29:00Z'
updated_date: '2026-11-07T11:39:00Z'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: high
type: bug
ordinal: 155000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Uživatel: uzly neustále překmitávají tam a zpět, to je strašné. Layout má spočítat cílovou polohu a do ní udělat plynulý přesun. Během tažení uzlu smí layout kmitat/hledat, ale jakmile uživatel uzel pustí, spočítá koncové polohy a uzly do nich plynule dojedou, bez skákání tam a zpět. Navazuje na TASK-77.28 (alpha cooling) a TASK-77.46 (plynulý přechod mezi layouty).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Mimo tažení se žádný uzel nepohybuje tam a zpět: po změně (start, sync, puštění uzlu) se spočte cílová poloha a uzly do ní dojedou plynulým monotónním přesunem
- [x] #2 Během tažení uzlu se ostatní uzly smí průběžně přizpůsobovat
- [x] #3 Po puštění uzlu se už žádný uzel nevrací přes svou cílovou polohu (žádný overshoot)
- [x] #4 Headless test prokáže, že dráha každého uzlu po puštění je monotónní k cíli a ustálí se
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Outside a drag the layout simulates a copy to rest (time-sliced Plan within the frame budget), then eases each unpinned node in a straight smoothstep to its rest position (0.3-4 s by distance); live simulation only while a node is dragged (layout.dragging from canvas.dragged_node). Any wake cancels a stale plan or settle move; Reset transitions keep running. Tests: test_layout_moves_monotonically_to_rest_at_start_and_after_release, test_task_77_60_layout_runs_live_only_while_a_node_is_dragged; budget tests now check the live path under drag and the planning step. Review: agent_review approved (non-blocking note: unbudgeted layout clone at plan start).
<!-- SECTION:FINAL_SUMMARY:END -->

---
id: TASK-77.60
title: >-
  Dashboard: uzly grafu nesmí překmitávat, po puštění plynulý přesun do spočtené
  cílové polohy
status: To Do
assignee: []
created_date: '2026-11-07T11:29:00Z'
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
- [ ] #1 Mimo tažení se žádný uzel nepohybuje tam a zpět: po změně (start, sync, puštění uzlu) se spočte cílová poloha a uzly do ní dojedou plynulým monotónním přesunem
- [ ] #2 Během tažení uzlu se ostatní uzly smí průběžně přizpůsobovat
- [ ] #3 Po puštění uzlu se už žádný uzel nevrací přes svou cílovou polohu (žádný overshoot)
- [ ] #4 Headless test prokáže, že dráha každého uzlu po puštění je monotónní k cíli a ustálí se
<!-- AC:END -->

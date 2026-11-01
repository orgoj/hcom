---
id: TASK-77.10
title: 'Persistence stavu grafu: ukládání a obnova do graph-state.json'
status: To Do
assignee: []
created_date: '2026-11-01 14:54'
labels: []
dependencies: []
parent_task_id: TASK-77
type: feature
ordinal: 88000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implementace ukládání a obnovy stavu aplikace do ~/.hcom/graph-state.json: pozice ručně pinovaných a přesunutých uzlů, zoom, posun kamery (pan), šířka a zobrazení postranního panelu, aktivní filtry a zvolený režim layoutu.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Pozice pinovaných a ručně přesunutých uzlů se ukládají a obnovují po restartu aplikace
- [ ] #2 Zvolený zoom, pozice kamery (pan) a šířka/viditelnost postranního panelu se persistují
- [ ] #3 Stav se ukládá do ~/.hcom/graph-state.json bez kolize s konfigurací nebo hcom-gui
- [ ] #4 Poškozený nebo nevalidní graph-state.json nezpůsobí pád a bezpečně použije výchozí hodnoty
<!-- AC:END -->

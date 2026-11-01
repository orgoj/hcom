---
id: TASK-77
title: 2D grafový dashboard pro hcom agenty (egui)
status: To Do
assignee: []
created_date: '2026-11-01 11:21'
labels: []
dependencies: []
type: feature
ordinal: 78000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Vizuální 2D dashboard pro sledování běžících hcom agentů, hierarchie spuštění (kdo koho pustil) a vzájemné komunikace (počet a směr zpráv, aktivita v reálném čase). Implementováno jako lehká samostatná Rust binárka (egui/eframe) bez závislosti na Electronu/webu.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Samostatná Rust aplikace spustitelná jako CLI/GUI binárka (např. contrib/hcom-graph)
- [ ] #2 Zobrazuje 2D cyklický graf s uzly (agenti) a orientovanými hranami (zprávy a vazby spuštění)
- [ ] #3 Barvy a stav uzlů odpovídají stavu agenta v hcom (active, listening, blocked, stopped)
- [ ] #4 Hrany znázorňují směr a počet zpráv mezi agenty
- [ ] #5 Kliknutí na agenta aktivuje jeho terminál (attach) stejně jako v hcom-gui
<!-- AC:END -->

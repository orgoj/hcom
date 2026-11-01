---
id: TASK-77.2
title: '2D grafové plátno: renderování uzlů, stavů a komunikačních toků v egui'
status: In Progress
assignee:
  - '@hcom_dev'
created_date: '2026-11-01 11:21'
updated_date: '2026-11-01 12:49'
labels: []
dependencies: []
parent_task_id: TASK-77
type: feature
ordinal: 80000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Vizuální komponenta v egui/eframe s využitím egui_graphs a petgraph pro 2D graf. Zobrazuje agenty jako interaktivní uzly obarvené podle stavu, animované/orientované šipky zpráv a stromové vazby spuštění s podporou pro zachycení kliknutí a pravého kliknutí.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Uzly jsou vykresleny s názvy a barvou odpovídající stavu (active, listening, blocked, stopped)
- [x] #2 Komunikační hrany zobrazují směr a počet zpráv mezi agenty
- [x] #3 Podpora zoomu (kolečko myši) a posunu plátna (pan)
- [x] #4 Uzel lze myší uchopit a přesunout (drag & drop pozicování)
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Background worker bez deadlocku: StreamReader (drzen vyhradne worker vlaknem bez zamku) a StreamKillHandle v samostatnem mutexu; drop zavola kill() a reap/wait na child i join na reader vlakno bez zombii a deadlocku; test podminen pro Unix (sleep 60) i Windows (powershell Start-Sleep 60) overi join <200ms.
2. Deterministicky coalescing pro 10k eventu: O(1) mezipamet (buffer drzi nejvyse 1 snapshot Option<AgentGraph>); worker cte v davkach a publikuje throttlovane na 16ms/200 eventu; test asertuje published_count <= 55 pri 10k eventech a prave 1 finalni konzistentni graf v bufferu.
3. Zmereny layout frame budget na hostu: 17 uzlu = 2.36 us/krok, 50 uzlu = 11.21 us, 100 uzlu = 34.16 us; frame budget stanoven na max 2ms per frame, max 2 sub-kroky, dt clamp [0.001, 0.033]s, usnuti pri energii pod prahem.
4. Drag & drop pinning lifecycle: behem dragu pinned=true a pozice sleduje kurzor (vel=0); pri mouse-up (uvolneni) pinned=false a uzel se plynule dotvaruje v poli.
5. Canvas rendering a interaktivita: status barvy (active green, listening blue, blocked red, stopped gray), Bezier orientovane message edges s badge poctu zprav, zoom (kolecko ukotvene ke kurzoru), pan.
6. Dokumentace a overitelne spusteni: cargo run --manifest-path contrib/hcom-graph/Cargo.toml v contrib/README.md.
<!-- SECTION:PLAN:END -->

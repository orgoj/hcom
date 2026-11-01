---
id: TASK-77.2
title: '2D grafové plátno: renderování uzlů, stavů a komunikačních toků v egui'
status: In Progress
assignee:
  - '@hcom_dev'
created_date: '2026-11-01 11:21'
updated_date: '2026-11-01 12:25'
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
- [ ] #1 Uzly jsou vykresleny s názvy a barvou odpovídající stavu (active, listening, blocked, stopped)
- [ ] #2 Komunikační hrany zobrazují směr a počet zpráv mezi agenty
- [ ] #3 Podpora zoomu (kolečko myši) a posunu plátna (pan)
- [ ] #4 Uzel lze myší uchopit a přesunout (drag & drop pozicování)
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Background worker bez deadlocku: rozdeleni na StreamReader (drzen vyhradne worker vlaknem bez zamku) a Arc<Mutex<Option<Box<dyn StreamKillHandle>>>> (drzen kill handlem); pri dropu BackgroundSync okamzite zavola kill() na handle bez blokovani mutexu readeru, coz okamzite zrusi rouru a odblokuje read_line (<100ms, overeno testem s realnym sleep childem).
2. Coalescing cadence pro 10k eventu: worker aplikuje eventy lokalne; publikace do Arc<Mutex<Option<AgentGraph>>> probiha po dokonceni davky nebo throttlovana kadenci 16ms/200 eventu; O(1) pamet i CPU, overeno testem test_overload_10k_events_throttled_coalescing.
3. Zmereny layout frame budget na hostu: 17 uzlu = 2.36 us/krok, 50 uzlu = 11.21 us, 100 uzlu = 34.16 us; frame budget stanoven na max 2ms per frame, max 2 sub-kroky, dt clamp [0.001, 0.033]s, usnuti pri energii pod prahem.
4. Drag & drop pinning lifecycle: behem dragu pinned=true a pozice sleduje kurzor (vel=0); pri mouse-up (uvolneni) pinned=false a uzel se plynule dotvaruje v poli.
5. Canvas rendering a interaktivita: status barvy (active green, listening blue, blocked red, stopped gray), Bezier orientovane message edges s badge poctu zprav, zoom (kolecko ukotvene ke kurzoru), pan.
6. Dokumentace a overitelne spusteni: cargo run --manifest-path contrib/hcom-graph/Cargo.toml v contrib/README.md.
<!-- SECTION:PLAN:END -->

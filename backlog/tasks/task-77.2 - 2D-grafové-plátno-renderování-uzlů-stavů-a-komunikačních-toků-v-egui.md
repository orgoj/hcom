---
id: TASK-77.2
title: '2D grafové plátno: renderování uzlů, stavů a komunikačních toků v egui'
status: In Progress
assignee:
  - '@hcom_dev'
created_date: '2026-11-01 11:21'
updated_date: '2026-11-01 12:20'
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
1. Background worker + mpsc channel: HcomSyncWorker bezi ve std::thread, UI thread (eframe::App::update) pouze neblokujici try_iter(), tichy stream nikdy nezmrazi GUI; child follower je drzen ve workeru a bezpecne ukoncen pri dropu/shutdown; unit test overi neblokujici update bez eventu.
2. Layout frame budget & dt clamp: zmereno 17 agentu na hostu; dt clampovano na [0.001, 0.033]s, max 2 sub-kroky simulace (max 2ms per frame); po vycerpani kineticke energie simulace usina (0ms).
3. Drag & drop pinning lifecycle: behem dragu pinned=true a pozice sleduje kurzor (vel=0); pri mouse-up (uvolneni) pinned=false a uzel se plynule dotvaruje v pruzinovem poli.
4. Canvas rendering a interaktivita: status barvy (active green, listening blue, blocked red, stopped gray), Bezier orientovane message edges s badge poctu zprav, zoom (kolecko ukotvene ke kurzoru), pan.
5. Dokumentace: kapitola v contrib/README.md s popisem behu a ovladani (cargo run -p hcom-graph).
<!-- SECTION:PLAN:END -->

---
id: TASK-77.2
title: '2D grafové plátno: renderování uzlů, stavů a komunikačních toků v egui'
status: In Progress
assignee:
  - '@hcom_dev'
created_date: '2026-11-01 11:21'
updated_date: '2026-11-01 12:18'
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
1. src/layout.rs: Force-directed / spring layout (Coulomb repulsion, Hooke springs on Spawn/Message edges, central gravity, damping, velocity cap). 100% unit testable without GUI.
2. src/canvas.rs: CanvasView with pan & zoom (scroll wheel anchored to cursor), node rendering with status colors (active: green, listening: blue, blocked: red/orange, stopped: gray), message edge arrows with message count badges, quadratic Bezier curvature for bidirectional message edges, drag & drop node positioning with pinning.
3. src/app.rs & src/main.rs: HcomGraphApp implementing eframe::App, dark theme, top status bar (agent counts, sync status, zoom controls), CentralPanel with CanvasView, HcomSyncWorker event polling.
4. Acceptance criteria verification: unit tests for layout physics, coordinate transforms (world_to_screen/screen_to_world), hit testing, drag-pinning; cargo fmt, clippy and tests passing.
<!-- SECTION:PLAN:END -->

---
id: TASK-77.45
title: 'hcom-dashboard: šipky zpráv podle objemu a samostatné vypínání vrstev hran'
status: To Do
assignee: []
created_date: '2026-11-06T19:45:00Z'
updated_date: '2026-11-06T20:57:00Z'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: medium
type: feature
ordinal: 138000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Uživatel (2026-11-06): šipky zpráv v obou směrech mají být úměrné toku dat mezi agenty (dnes tloušťka roste log2 s počtem zpráv v okně a od 32 zpráv se nemění, canvas.rs). Hlavní šipky zahájení (TASK-77.44), šipky zpráv a spawn hrany má jít vypínat zvlášť: jednou jen kdo zahájil, jindy jen toky dat.

Rozhodnutí uživatele (2026-11-06): přepínatelná metrika Počet/Bajty (bajty = délka textu zpráv v časovém okně); šířka 1–10 px podle sqrt(hodnota / maximum mezi viditelnými hranami zpráv); přepínače vrstev Start, Msgs, Spawn v řádku 2 za Layout jako selectable labels; vrstvy i metrika se pamatují v dashboard-state.json. Vypnutí Msgs pokrývá TASK-77.25 AC #3 (hrany zpráv v Hierarchy).

V rozsahu: metrika, škála, tři přepínače vrstev, persistence, legenda nekreslí vypnuté vrstvy. Mimo rozsah: badge s počtem zůstává počet zpráv; vypnuté vrstvy neovlivňují layout (síly), jen kreslení a klikání; žádné jiné změny Hierarchy.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Přepínač Msgs/Bytes volí hodnotu: počet zpráv nebo součet bajtů textu zpráv hrany v okně
- [ ] #2 Přepínače Start, Msgs a Spawn v řádku 2 za Layout zapínají a vypínají kreslení šipek zahájení, šipek zpráv a spawn hran nezávisle
- [ ] #3 Vypnutá vrstva se nekreslí, její hrany nejde najet ani kliknout a legenda ji neuvádí; layout se nemění
- [ ] #4 Stav přepínačů a metriky přežije restart (dashboard-state.json); výchozí stav: všechny vrstvy zapnuté, metrika Msgs
- [ ] #5 Regresní testy pokrývají škálu, obě metriky, vypnutí každé vrstvy (kreslení/hit test/legenda) a persistenci
- [ ] #6 Tloušťka šipky zprávy je při zoomu 100 % 1–10 px podle sqrt(hodnota/maximum) přes viditelné hrany zpráv v časovém okně; nejsilnější hrana má 10 px
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. WindowedCount += bytes; windowed_edge_counts sčítá text.len() (i v neomezeném okně z historie). 2. DashboardState: show_initiator_edges, show_message_edges, show_spawn_edges (default true), edge_width_metric (count|bytes). 3. CanvasView dostane vrstvy a metriku; thickness = 1 + 9*sqrt(v/max) * zoom clamp; vypnuté vrstvy přeskočí kreslení a hit_test_edge_filtered. 4. legend() respektuje vrstvy. 5. Řádek 2: 'Edges:' Start/Msgs/Spawn + 'Width:' Msgs/Bytes, save_state. 6. Testy.
<!-- SECTION:PLAN:END -->

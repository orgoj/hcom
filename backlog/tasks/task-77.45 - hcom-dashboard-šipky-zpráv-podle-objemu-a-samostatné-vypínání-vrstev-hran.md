---
id: TASK-77.45
title: 'hcom-dashboard: šipky zpráv podle objemu a samostatné vypínání vrstev hran'
status: Done
assignee:
  - '@hcom_dev__idea'
created_date: '2026-11-06T19:45:00Z'
updated_date: '2026-11-06T21:03:00Z'
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
- [x] #1 Přepínače Start, Msgs a Spawn v řádku 2 za Layout zapínají a vypínají kreslení šipek zahájení, šipek zpráv a spawn hran nezávisle
- [x] #2 Vypnutá vrstva se nekreslí, její hrany nejde najet ani kliknout a legenda ji neuvádí; layout se nemění
- [x] #3 Regresní testy pokrývají škálu, obě metriky, vypnutí každé vrstvy (kreslení/hit test/legenda) a persistenci
- [x] #4 Přepínač Width Count/Bytes volí hodnotu: počet zpráv nebo součet bajtů textu zpráv hrany v okně
- [x] #5 Stav přepínačů a metriky přežije restart (dashboard-state.json); výchozí stav: všechny vrstvy zapnuté, metrika Count
- [x] #6 Tloušťka šipky zprávy je při zoomu 100 % 1–10 px podle sqrt(hodnota/maximum) přes viditelné hrany zpráv v časovém okně; nejsilnější hrana má 10 px; stávající zvýraznění najetí (+1 px) a výběru (+2 px) se přičítá jako dosud
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. WindowedCount += bytes; windowed_edge_counts sčítá text.len() (i v neomezeném okně z historie). 2. DashboardState: show_initiator_edges, show_message_edges, show_spawn_edges (default true), edge_width_metric (count|bytes). 3. CanvasView dostane vrstvy a metriku; thickness = 1 + 9*sqrt(v/max) * zoom clamp; vypnuté vrstvy přeskočí kreslení a hit_test_edge_filtered. 4. legend() respektuje vrstvy. 5. Řádek 2: 'Edges:' Start/Msgs/Spawn + 'Width:' Msgs/Bytes, save_state. 6. Testy.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Width metric toggle is labelled Count/Bytes ("Msgs" is the message layer toggle); AC #5/#6 reworded. Width max is taken over the drawn message edges (layer on, both ends visible and laid out). Layers only gate drawing, hit test and legend; layout forces are unchanged.

Review finding (hover/selection exceed 10 px) rejected: the +1/+2 px is the existing interaction highlight, not flow width; AC reworded to state it.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Message arrows: base width 1 + 9*sqrt(value/max) px at zoom 1 over drawn message edges, value = count or text bytes (Width: Count/Bytes; MessageEdge.bytes, WindowedCount.bytes). Edge layers Start/Msgs/Spawn in row 2 after Layout gate drawing, hit test and legend (Legend::apply_layers); layout unchanged. State persisted in dashboard-state.json (defaults all on, Count). Verified with 6 task_77_45 tests, fmt, clippy 0 warnings, tests 17+177 passed. Review finding (hover/selection width) rejected, AC clarified.
<!-- SECTION:FINAL_SUMMARY:END -->

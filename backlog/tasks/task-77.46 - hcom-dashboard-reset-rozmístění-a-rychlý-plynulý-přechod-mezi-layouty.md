---
id: TASK-77.46
title: 'hcom-dashboard: reset rozmístění a rychlý plynulý přechod mezi layouty'
status: Idea
assignee: []
created_date: '2026-11-06T20:05:00Z'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: medium
type: feature
ordinal: 139000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Uživatel (2026-11-06): rozházení uzlů se mu nelíbí; osamělé uzly bez hran zůstávají daleko od ostatních; při přepnutí z Hierarchy (roztažené do řady) na Cluster vzdálené uzly dojíždějí nekonečně pomalu. Chce reset rozmístění, který graf přeskládá optimálněji, a progresivní rychlost: zrychlit, pak zpomalit a rychle se ustálit. Chvění se už zastavuje (TASK-77.28), to je v pořádku. Příčina pomalosti (layout.rs): každá síla se násobí alpha, která po set_mode klesá z 1 k alpha_min za ~900 kroků, a max_speed 270 px/s; vzdálený uzel nedojede, než síla vyhasne. Navržený směr: cílové rozmístění spočítat dopředu bez kreslení (simulace do ustálení, piny zůstávají) a uzly k němu animovat s ease-in-out (~0,5–1 s); totéž použít pro přepnutí režimu i pro tlačítko Reset rozmístění; osamělé uzly v cílovém rozmístění blízko ostatních (např. mřížka pod/vedle komponent). Otevřené: umístění tlačítka Reset (vedle Fit?), zda reset ruší piny, délka animace. Needs analysis before implementation.
<!-- SECTION:DESCRIPTION:END -->

---
id: TASK-77.45
title: 'hcom-dashboard: šipky zpráv podle objemu a samostatné vypínání vrstev hran'
status: Idea
assignee: []
created_date: '2026-11-06T19:45:00Z'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: medium
type: feature
ordinal: 138000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Uživatel (2026-11-06): šipky zpráv v obou směrech mají být úměrné toku dat mezi agenty, podle počtu zpráv nebo jejich velikosti (dnes tloušťka roste logaritmicky s počtem zpráv v okně, canvas.rs). Hlavní šipky zahájení (TASK-77.44) a šipky zpráv má jít vypínat zvlášť: jednou chce vidět jen kdo zahájil, jindy jen toky dat. Otevřené: počet vs. velikost (bajty textu) nebo přepínatelné; kde v UI jsou přepínače a zda se pamatují v dashboard-state.json; vztah ke spawn hranám. Závisí na TASK-77.44. Needs analysis before implementation.
<!-- SECTION:DESCRIPTION:END -->

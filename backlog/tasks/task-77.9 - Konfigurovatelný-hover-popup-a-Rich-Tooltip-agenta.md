---
id: TASK-77.9
title: Konfigurovatelný hover popup a Rich Tooltip agenta
status: To Do
assignee: []
created_date: '2026-11-01 14:53'
labels: []
dependencies: []
parent_task_id: TASK-77
type: feature
ordinal: 87000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Při najetí myší (hover) na uzel agenta zobrazit plovoucí informační kartu / Rich Tooltip s klíčovými metrikami (PID, CLI, model, reasoning effort, cwd/projekt, věk, unread zprávy, zaplnění kontextu, aktuální stav a detail). Konfigurace zobrazovaných polí v JSON souboru (např. ~/.hcom/graph.json), obdobně jako u Report tabu v hcom-gui.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Najetí myší na uzel agenta otevře přehledný plovoucí popup / tooltip s metrikami agenta
- [ ] #2 Tooltip zobrazuje PID, CLI typ, model, reasoning effort, pracovní adresář, věk a stav
- [ ] #3 Zobrazovaná pole a jejich pořadí jsou konfigurovatelné přes JSON konfiguraci
- [ ] #4 Tooltip se nezasekává, respektuje hranice okna plátna a plynule mizí při opuštění uzlu
<!-- AC:END -->

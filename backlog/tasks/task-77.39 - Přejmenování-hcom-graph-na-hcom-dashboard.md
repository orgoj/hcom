---
id: TASK-77.39
title: Přejmenování hcom-graph na hcom-dashboard
status: To Do
assignee: []
created_date: '2026-11-06T17:33:00Z'
updated_date: '2026-11-06T17:36:00Z'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: medium
type: enhancement
ordinal: 132000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Aplikace přerostla původní pasivní graf: má tabulku agentů, detail panel (Screen, Transcript, Events, Log, Config), akce Start/Send/Attach/Restart/Kill a nahrazuje Python hcom-gui. Název hcom-graph to už nepopisuje. Přejmenovat adresář, crate, binárku, konfiguraci, stav a dokumentaci na hcom-dashboard. Rozhodnutí uživatele: bez zpětné kompatibility a migrace starých souborů, aplikaci zatím používá jen autor forku; kompatibilita přijde na řadu až s číslovanými releasy a changelogem.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Adresář contrib/hcom-dashboard, crate i binárka se jmenují hcom-dashboard a titulek okna uvádí hcom-dashboard
- [ ] #2 Konfigurace se čte jen z ~/.hcom/dashboard.json; graph.json ani gui.json se nečtou, bez dashboard.json platí vestavěné výchozí hodnoty
- [ ] #3 Stav se čte a ukládá jen v ~/.hcom/dashboard-state.json; graph-state.json ani gui-state.json se nečtou a kód jejich fallbacku a převodu starých klíčů je odstraněný
- [ ] #4 README aplikace, contrib/README.md a testy používají jen nový název a nové soubory, bez zmínek o hcom-graph a starých souborech
<!-- AC:END -->

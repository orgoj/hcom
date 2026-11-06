---
id: TASK-77.39
title: Přejmenování hcom-graph na hcom-dashboard
status: To Do
assignee: []
created_date: '2026-11-06T17:33:00Z'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: medium
type: enhancement
ordinal: 132000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Aplikace přerostla původní pasivní graf: má tabulku agentů, detail panel (Screen, Transcript, Events, Log, Config), akce Start/Send/Attach/Restart/Kill a nahrazuje Python hcom-gui. Název hcom-graph to už nepopisuje. Přejmenovat adresář, crate, binárku, konfiguraci, stav a dokumentaci na hcom-dashboard tak, aby uživateli nezmizelo nastavení ani uložený layout.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Adresář contrib/hcom-dashboard, crate i binárka se jmenují hcom-dashboard a titulek okna uvádí hcom-dashboard
- [ ] #2 Konfigurace se čte z ~/.hcom/dashboard.json, při jeho absenci z ~/.hcom/graph.json a pak z ~/.hcom/gui.json
- [ ] #3 Stav se ukládá do ~/.hcom/dashboard-state.json; když neexistuje, načte se jednou z graph-state.json (pak gui-state.json), takže pozice, splity a filtry zůstanou zachované; staré soubory se nepřepisují
- [ ] #4 README aplikace, contrib/README.md a testy používají nový název; hcom-graph zůstává jen v popisu migrace starých souborů
<!-- AC:END -->

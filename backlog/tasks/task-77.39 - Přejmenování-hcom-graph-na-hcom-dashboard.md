---
id: TASK-77.39
title: Přejmenování hcom-graph na hcom-dashboard
status: Done
assignee:
  - '@hcom_dev__idea'
created_date: '2026-11-06T17:33:00Z'
updated_date: '2026-11-06T18:47:00Z'
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
- [x] #1 Adresář contrib/hcom-dashboard, crate i binárka se jmenují hcom-dashboard a titulek okna uvádí hcom-dashboard
- [x] #2 Konfigurace se čte jen z ~/.hcom/dashboard.json; graph.json ani gui.json se nečtou, bez dashboard.json platí vestavěné výchozí hodnoty
- [x] #3 Stav se čte a ukládá jen v ~/.hcom/dashboard-state.json; graph-state.json ani gui-state.json se nečtou a kód jejich fallbacku a převodu starých klíčů je odstraněný
- [x] #4 README aplikace, contrib/README.md a testy používají jen nový název a nové soubory; staré soubory zmiňují jen testy, které dokazují, že se nečtou ani nepřepisují
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Renamed contrib/hcom-graph to contrib/hcom-dashboard (crate, binary, window title, HcomDashboardApp/DashboardConfig/DashboardState). Config only ~/.hcom/dashboard.json, state only dashboard-state.json; graph.json/gui.json/graph-state.json/gui-state.json fallbacks and legacy key conversions removed with their tests (user: no backward compatibility before numbered releases). Tests test_config_reads_only_dashboard_json and test_state_reads_and_saves_only_dashboard_state_json. Review agent_review OK.
<!-- SECTION:FINAL_SUMMARY:END -->

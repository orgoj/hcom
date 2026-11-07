---
id: TASK-77.56
title: 'Dashboard: dialog Start stejný jako odladěný dialog v hcom-gui'
status: To Do
assignee: []
created_date: '2026-11-07T08:42:00Z'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: high
type: bug
ordinal: 151000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Dialog Start v dashboardu (tlačítko Start) neodpovídá odladěnému dialogu Start v contrib/hcom-gui. Rozdíly hlášené uživatelem: neotevírá se uprostřed obrazovky; chybí rozbalovací (popup) volby jako v hcom-gui; jméno agenta se předvyplní jen po kliknutí (fokusu) do pole jména. Vzorem je chování dialogu v hcom-gui. Souvisí s TASK-77.55 (otevření dialogu dvojklikem/Enterem).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Dialog Start se otevře uprostřed obrazovky
- [ ] #2 Dialog nabízí stejné rozbalovací volby jako dialog Start v hcom-gui
- [ ] #3 Jméno agenta je předvyplněné hned po otevření dialogu, bez fokusu do pole
- [ ] #4 Ostatní pole a jejich předvyplnění se chovají jako v dialogu Start v hcom-gui
<!-- AC:END -->

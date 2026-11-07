---
id: TASK-77.55
title: 'Dashboard: start agentů nesmí blokovat UI'
status: To Do
assignee: []
created_date: '2026-11-07T08:10:00Z'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: high
type: bug
ordinal: 150000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Spuštění agentů z dashboardu blokuje aplikaci (UI nereaguje, dokud start neskončí).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Během startu agenta dashboard dál reaguje na vstup a překresluje se
- [ ] #2 Výsledek startu (úspěch/chyba) se zobrazí po dokončení bez zamrznutí UI
<!-- AC:END -->

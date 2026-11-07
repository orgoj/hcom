---
id: TASK-77.55
title: 'Dashboard: dvojklik nebo Enter na nespuštěném agentovi neotevře dialog Start'
status: To Do
assignee: []
created_date: '2026-11-07T08:10:00Z'
updated_date: '2026-11-07T08:40:00Z'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: high
type: bug
ordinal: 150000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Dvojklik nebo Enter na řádku agenta, který neběží, nic neudělá: neotevře se dialog Start. U běžícího agenta stejná akce agenta připojí (TASK-77.50). Upřesnění uživatele: nejde o blokování aplikace.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Dvojklik na řádek nespuštěného agenta otevře dialog Start, stejný jako tlačítko Start
- [ ] #2 Enter na vybraném nespuštěném agentovi otevře dialog Start
- [ ] #3 U běžícího agenta dvojklik i Enter dál agenta připojí (attach)
<!-- AC:END -->

---
id: TASK-77.80
title: 'Dashboard: prázdný stav při selhání načtení agentů'
status: To Do
assignee: []
created_date: '2026-11-07T15:57:00Z'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: medium
type: enhancement
ordinal: 175000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
UX review 2026-11-07 (F4, A11; Q5 rozhodnuto: binárka hcom se nemění, jeden nedostupný import katalogu smí shodit fetch, řeší se jen v dashboardu). Při selhání fetch zůstane graf i tabulka prázdná (Shown 0 / N), důvod jen v Logu ([ERROR] Failed to fetch live/agent data ... ×N). Řešení: text v těle tabulky a ve středu plátna 'Agent data could not be loaded — see Log (F8)' v barvě chybového textu, jeden řádek; zmizí s dalším úspěšným fetchem.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Při selhaném fetchi agentů ukazuje tělo tabulky i plátno text Agent data could not be loaded — see Log (F8)
- [ ] #2 Po dalším úspěšném fetchi text zmizí
<!-- AC:END -->

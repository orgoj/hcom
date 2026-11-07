---
id: TASK-77.76
title: 'Dashboard: filtr Running only podle časového okna'
status: To Do
assignee: []
created_date: '2026-11-07T14:59:00Z'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: medium
type: feature
ordinal: 171000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Uživatel 2026-11-07: Running only má zahrnout všechny agenty, kteří v nastaveném časovém okně běželi, ne jen právě běžící. Data: life eventy created/stopped z ReloadStats (compute_event_stats).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 S Running only tabulka i graf ukazují agenty běžící teď a agenty, jejichž některá sešna (created..stopped) zasahuje do časového okna zpráv
- [ ] #2 Okno All zahrne každého agenta s nějakou sešnou v načtené historii
- [ ] #3 Testy pokrývají agenta běžícího v okně, mimo okno a právě běžícího
<!-- AC:END -->

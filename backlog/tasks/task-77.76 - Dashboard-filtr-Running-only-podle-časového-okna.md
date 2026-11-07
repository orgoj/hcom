---
id: TASK-77.76
title: 'Dashboard: filtr Running only podle časového okna'
status: Done
assignee:
  - '@hcom_dev__idea'
created_date: '2026-11-07T14:59:00Z'
updated_date: '2026-11-07T15:09:00Z'
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
- [x] #1 S Running only tabulka i graf ukazují agenty běžící teď a agenty, jejichž některá sešna (created..stopped) zasahuje do časového okna zpráv
- [x] #2 Okno All zahrne každého agenta s nějakou sešnou v načtené historii
- [x] #3 Testy pokrývají agenta běžícího v okně, mimo okno a právě běžícího
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Running only = běží teď nebo některý interval created..stopped (z ReloadStats) zasahuje do okna zpráv; tabulka i graf. Test test_task_77_76_running_only_follows_the_window; review bez nálezů.
<!-- SECTION:FINAL_SUMMARY:END -->

---
id: TASK-77.43
title: 'Review TASK-77.38: created po batch_launched téhož spuštění'
status: Done
assignee: []
created_date: '2026-11-06T19:29:00Z'
labels:
  - review:rejected
dependencies: []
parent_task_id: TASK-77
priority: low
type: chore
ordinal: 136000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
agent_review hlásil, že created dítěte může přijít po batch_launched téhož spuštění a smazat spouštěče.
<!-- SECTION:DESCRIPTION:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Zamítnuto: spouštěč loguje created dítěte synchronně v cyklu spouštění (src/launcher.rs initialize_launch_instance → instance_binding log_created_and_auto_subscribe) před batch_launched po cyklu; pozdější binding dítěte created znovu neloguje (save_instance_named jen pro nový řádek). Živá data: 3015/3015 spuštění mají nejbližší created před batch_launched. Předpoklad je okomentován v compute_event_stats.
<!-- SECTION:FINAL_SUMMARY:END -->

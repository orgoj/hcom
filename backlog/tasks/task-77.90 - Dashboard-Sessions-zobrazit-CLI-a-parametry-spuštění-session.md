---
id: TASK-77.90
title: 'Dashboard Sessions: zobrazit CLI a parametry spuštění session'
status: Idea
assignee: []
created_date: '2026-11-07T17:54:00Z'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: medium
type: feature
ordinal: 185000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Uživatel 2026-11-07: v záložce Sessions není vidět, jakým agent CLI byla session spuštěna, takže uživatel nepozná, kterou session chce. Přání: u každé session zobrazit parametry, se kterými byla spuštěna (CLI, argumenty). Suggested direction: zjistit, co o session ukládá hcom (life events / instance řádek: tool, launch args) a co Sessions tab dnes načítá (ActionRequest::Sessions v action.rs, sessions.rs). Needs analysis before implementation.
<!-- SECTION:DESCRIPTION:END -->

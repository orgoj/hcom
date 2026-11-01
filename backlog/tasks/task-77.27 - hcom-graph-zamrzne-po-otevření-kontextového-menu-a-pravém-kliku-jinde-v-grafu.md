---
id: TASK-77.27
title: hcom-graph zamrzne po otevření kontextového menu a pravém kliku jinde v grafu
status: Idea
assignee: []
created_date: '2026-11-01 20:39'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: high
type: bug
ordinal: 105000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Uživatel zmrazil celé hcom-graph (po TASK-77.18, release 319f8bc3), proces musel zabít. Podle uživatele asi po rozbalení popup menu (kontextové menu uzlu, příp. menu režimu) a pravém kliku jinde v grafu. Přesná sekvence a příčina nejsou známé; proces už neběží, stav vláken nezjištěn. Suggested direction: zjistit spolehlivou reprodukci (headless test App::ui s pravými/levými kliky na uzly, prázdné plátno a otevřené menu), ověřit zda jde o zaseknutí smyčky (CPU) nebo blokované vstupy UI. Needs analysis before implementation.
<!-- SECTION:DESCRIPTION:END -->

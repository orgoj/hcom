---
id: TASK-77.98
title: Headless dump grafu dashboardu do JSON a Mermaid s filtry okna
status: To Do
assignee: []
created_date: '2026-11-09T07:31:00Z'
labels:
  - feature
dependencies: []
parent_task_id: TASK-77
priority: medium
ordinal: 194000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Uživatel: ladicí režim bez GUI, aby agent mohl graf kontrolovat sám. hcom-dashboard s CLI volbou (např. --dump json|mermaid) načte stejná data jako GUI (zprávy, session starts, instance) a vypíše graf podle zadaných filtrů (okno --window 6h / interval, --last-n, vrstvy Start/messages/spawn, filtr agentů) na stdout a skončí. Použití i jinde (export).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 --dump json vypíše uzly, hrany zpráv s počty v okně, šipky Start s iniciátory a spawn hrany
- [ ] #2 --dump mermaid vypíše tentýž graf jako Mermaid flowchart
- [ ] #3 Filtry okna a vrstev odpovídají GUI (stejný výpočet z model.rs, žádná duplikace)
- [ ] #4 Test v model_tests.rs a popis v contrib/README.md
<!-- AC:END -->

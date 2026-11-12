---
id: TASK-77.105
title: 'Dashboard: seskupení instancí pod agenty (mraky v grafu a agregace statistik)'
status: Done
assignee:
  - '@hcom_dev__dashboard'
created_date: '2026-11-12T10:58:00Z'
updated_date: '2026-11-12T12:19:00Z'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: high
type: feature
ordinal: 206000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
V grafu dashboardu zobrazit instance patřící ke stejnému agentovi seskupené u sebe v rámci jednoho mraku (cluster/compound bubble) s popiskem agenta. V dlouhodobých přehledech a statistikách agregovat data na úrovni agenta (instance jsou živé jen za běhu). V záložce Sessions a pro transcripty mapovat instance na mateřského agenta.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Instance patřící ke stejnému agentovi se v grafu drží u sebe (přitažlivá síla clusteru/centroidu)
- [ ] #2 Skupina instancí stejného agenta je na plátně vizuálně ohraničena jako mrak/bublina s názvem agenta
- [ ] #3 V záložce Sessions agenta se zobrazují relace a transkripty všech jeho instancí (včetně roaming/klonů)
- [ ] #4 Statistiky a souhrny zpráv v okně All lze prohlížet agregovaně za agenta
<!-- AC:END -->

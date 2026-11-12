---
id: TASK-77.107
title: >-
  Dashboard: tažení instance skáče a výpočet konečné polohy po puštění trvá
  dlouho
status: To Do
assignee: []
created_date: '2026-11-12T16:18:00Z'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: medium
type: bug
ordinal: 208000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Hlášeno uživatelem po TASK-77.106 (tuhá vazba instancí, 56435e7d). (1) Při ručním tažení uzlu instance v grafu silně skáče; podezření: bind_agent_groups v živé simulaci (dragging) přesouvá tažený/kotevní uzel nebo ostatní instance skupiny skokově. (2) Po puštění je dojezd plynulý, ale výpočet konečné polohy (plán) trvá velmi dlouho, přestože na zachycených živých datech (193 uzlů) trval plán 0,7 s čistého času; ověřit chování v aplikaci (rozpočet snímku, překreslování, počet uzlů v layoutu včetně zastavených z historie).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Tažení uzlu nezpůsobuje viditelné skoky instancí téhož agenta
- [ ] #2 Po puštění začne plynulý dojezd na konečnou polohu bez znatelného čekání
<!-- AC:END -->

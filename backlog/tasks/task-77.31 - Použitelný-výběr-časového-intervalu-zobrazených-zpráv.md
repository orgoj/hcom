---
id: TASK-77.31
title: Použitelný výběr časového intervalu zobrazených zpráv
status: To Do
assignee: []
created_date: '2026-11-02 21:38'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: high
type: enhancement
ordinal: 115000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Uživatel: dnešní filtr časového okna zpráv je jeden posuvník bez tahatelné levé a pravé strany a interval nejde zadat textem. Refaktorovat filtr tak, aby byl použitelný: posuvník rozsahu se dvěma úchyty a textové zadání. Překryv: TASK-77.26 AC #1 a #2 popisují popisky dnešního posuvníku; při implementaci je sladit nebo přepsat.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Posuvník intervalu má samostatně tahatelný začátek i konec
- [ ] #2 Do textového pole lze zadat relativní úsek (např. 1h = poslední hodina) a graf ukáže zprávy z tohoto úseku
- [ ] #3 Do textového pole lze zadat absolutní rozsah od-do a graf ukáže zprávy z tohoto rozsahu
- [ ] #4 Posuvník a textové pole ukazují vždy tentýž interval; neplatný zápis interval nezmění a je viditelně označený
<!-- AC:END -->

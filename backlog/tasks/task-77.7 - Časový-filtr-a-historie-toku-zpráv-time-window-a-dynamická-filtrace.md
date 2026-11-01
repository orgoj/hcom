---
id: TASK-77.7
title: 'Časový filtr a historie toku zpráv: time window a dynamická filtrace'
status: To Do
assignee: []
created_date: '2026-11-01 14:53'
updated_date: '2026-11-01 15:11'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: high
type: feature
ordinal: 84000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Přidání volby časového okna pro graf a historii zpráv: bohaté možnosti výběru (předvolby 5m, 15m, 1h, 6h, 24h, Vše), interaktivní časový slider (timeline scrubber) a možnost zadání přesného cílového časového rozsahu nebo počtu zpráv. Plátno dynamicky filtruje komunikační hrany podle zvoleného okna a v záhlaví přehledně zobrazuje aktuální časový interval.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Horní lišta nabízí rozšířené předvolby časového horizontu (5m, 15m, 1h, 6h, 24h, Vše)
- [ ] #2 K dispozici je interaktivní časový posuvník (slider / timeline scrubber) pro plynulé procházení historie
- [ ] #3 Možnost zadat vlastní cílový rozsah (časový filtr nebo limit zpráv)
- [ ] #4 Komunikační hrany, čítače zpráv a detail konverzace se dynamicky přizpůsobují zvolenému oknu
- [ ] #5 Hrany neaktivní ve zvoleném období jsou skryty nebo ztlumeny a v záhlaví je jasně indikován časový rozsah
<!-- AC:END -->

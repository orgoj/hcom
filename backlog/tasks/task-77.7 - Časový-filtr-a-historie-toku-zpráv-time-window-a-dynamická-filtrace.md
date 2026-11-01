---
id: TASK-77.7
title: 'Časový filtr a historie toku zpráv: time window a dynamická filtrace'
status: Done
assignee:
  - '@hcom_dev__idea'
created_date: '2026-11-01 14:53'
updated_date: '2026-11-01 18:29'
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
- [x] #1 Horní lišta nabízí rozšířené předvolby časového horizontu (5m, 15m, 1h, 6h, 24h, Vše)
- [x] #2 K dispozici je interaktivní časový posuvník (slider / timeline scrubber) pro plynulé procházení historie
- [x] #3 Možnost zadat vlastní cílový rozsah (časový filtr nebo limit zpráv)
- [x] #4 Komunikační hrany, čítače zpráv a detail konverzace se dynamicky přizpůsobují zvolenému oknu
- [x] #5 Hrany neaktivní ve zvoleném období jsou skryty nebo ztlumeny a v záhlaví je jasně indikován časový rozsah
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Plán i diff schválil agent_review. Opraveny nity: reset cache klíče při výměně grafu ze syncu, live okno bez horní hranice (clock skew), DragValue ukládá stav až po skončení drag/edit, lišta okna se zalamuje. Ponecháno: live okno jen s last_n se překresluje každou sekundu (neškodné).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Časové okno zpráv v hcom-graph: předvolby 5m–24h/Vše, vlastní délka v minutách a limit posledních N zpráv, timeline scrubber s Live, hrany/počty/detail konverzace podle okna, neaktivní hrany ztlumené a klikatelné, rozsah v UTC v záhlaví, N+ pro neúplné počty; span a limit se ukládají v graph-state.json. Ověřeno 7 novými testy (67/67), fmt, clippy 0 varování.
<!-- SECTION:FINAL_SUMMARY:END -->

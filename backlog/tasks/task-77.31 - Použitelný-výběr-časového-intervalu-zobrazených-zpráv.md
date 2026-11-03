---
id: TASK-77.31
title: Použitelný výběr časového intervalu zobrazených zpráv
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-11-02 21:38'
updated_date: '2026-11-03 14:48'
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
- [x] #1 Posuvník intervalu má samostatně tahatelný začátek i konec
- [x] #2 Do textového pole lze zadat relativní úsek (např. 1h = poslední hodina) a graf ukáže zprávy z tohoto úseku
- [x] #3 Do textového pole lze zadat absolutní rozsah od-do a graf ukáže zprávy z tohoto rozsahu
- [x] #4 Posuvník a textové pole ukazují vždy tentýž interval; neplatný zápis interval nezmění a je viditelně označený
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Implementován interaktivní posuvník se dvěma táhly a textové zadávání intervalu zpráv (relativní např. 1h, 15m, all i absolutní od-do např. 12:00 - 13:00, 2026-11-03 12:00 - 2026-11-03 13:00, all - 13:00, 12:00 - now). Neplatný formát je indikován červeným rámečkem bez narušení existujícího stavu okna. Potvrzení přes Enter/ztrátu fokusu provádí synchronizaci na kanonický tvar a perzistenci do graph-state.json. Ošetřeno splynutí táhel, směrové tažení a maximální rozsah do 100 let. Pokryto sadou integračních testů a zdokumentováno v README.
<!-- SECTION:FINAL_SUMMARY:END -->

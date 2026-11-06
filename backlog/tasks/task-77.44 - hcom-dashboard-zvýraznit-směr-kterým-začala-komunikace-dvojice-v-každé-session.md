---
id: TASK-77.44
title: >-
  hcom-dashboard: zvýraznit směr, kterým začala komunikace dvojice v každé
  session
status: To Do
assignee: []
created_date: '2026-11-06T19:41:00Z'
updated_date: '2026-11-06T19:45:00Z'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: medium
type: feature
ordinal: 137000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Mezi dvěma agenty jsou dnes šipky zpráv v obou směrech (tloušťka podle počtu zpráv); ty zůstávají beze změny. Chybí hlavní šipka, která ukáže, kdo komunikaci zahájil: u agenta, kterého se všichni ptají (např. knowledge), nelze poznat, že začali tazatelé. Rozhodnutí uživatele (2026-11-06): komunikace dvojice A–B se dělí na úseky, nový úsek začíná novou session kohokoli z dvojice (life event created); iniciátor úseku = odesílatel první zprávy v úseku, která není odpovědí (reply_to, intent ack). Hlavní šipka je samostatná, vede od iniciátora; když úseky zahájily obě strany, má hroty na obou koncích. Mimo rozsah: tloušťka šipek zpráv a vypínání vrstev (samostatný task), strom spuštění (TASK-77.38).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Šipky zpráv dál ukazují oba směry, jednosměrná komunikace má jen jednu šipku
- [ ] #2 Komunikace dvojice se dělí na úseky; nový úsek začne life eventem created kteréhokoli z dvojice
- [ ] #3 Detail hrany a tooltip uvádějí iniciátora (u více úseků všechny s počtem)
- [ ] #4 Iniciátor úseku je odesílatel první zprávy v úseku, která není odpovědí: zprávy s reply_to nebo s intent ack úsek nezahajují; úsek bez takové zprávy nemá iniciátora
- [ ] #5 Regresní testy pokrývají jednosměrnou komunikaci, zahájení každou stranou, nový úsek po created jedné strany, odpověď (reply_to i ack) jako první zprávu úseku a oba zvýrazněné směry
- [ ] #6 Mezi dvojicí, kde aspoň jeden úsek má iniciátora, je samostatná hlavní šipka (oddělená od šipek zpráv) směrem od iniciátora; zahájily-li úseky obě strany, má hlavní šipka hroty na obou koncích
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Uživatel (2026-11-06): odpovědi (reply_to, intent ack) konverzaci nezahajují; např. po restartu agenta mu druhý teprve odpovídá na dřívější dotaz, to není začátek konverzace.
<!-- SECTION:NOTES:END -->

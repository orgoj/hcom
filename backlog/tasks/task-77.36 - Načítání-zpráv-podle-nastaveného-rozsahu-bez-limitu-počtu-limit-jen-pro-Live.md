---
id: TASK-77.36
title: Načítání zpráv podle nastaveného rozsahu bez limitu počtu (limit jen pro Live)
status: Done
assignee:
  - '@hcom_dev__idea'
created_date: '2026-11-03 15:30'
updated_date: '2026-11-03 16:07'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: high
type: enhancement
ordinal: 120000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Uživatel: Dnes se při startu hcom-graph načítá pevný počet posledních zpráv (500) a každá hrana drží nanejvýš 50 zpráv, což způsobuje, že při přepnutí filtru na 'all' nebo při nastavení širšího časového okna chybí starší vazby mezi uzly. Limit na počet zpráv má platit výhradně pro režim Live (případně explicitní filtr 'Last: N'). Pokud uživatel nastaví časové období (např. all, nebo rozsah od-do), nesmí existovat žádný umělý limit na počet zpráv a graf musí načíst a zobrazit všechny zprávy a vazby spadající do zvoleného časového intervalu.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Při nastavení časového rozsahu (např. 'all' nebo interval od-do) se načtou a zobrazí všechny zprávy spadající do daného intervalu bez omezení na výchozí snapshot limit 500 zpráv
- [x] #2 Všechny komunikační hrany v daném intervalu mají přesné počty a historii odpovídající všem uloženým zprávám v databázi
- [x] #3 Limit na počet zpráv se uplatňuje výhradně při explicitním zadání filtru 'Last: N'; režim Live (ani Live + 'all') nemá žádný implicitní limit počtu
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
See review plan: load all messages via follower replay from after-id 0, drop MAX_EDGE_HISTORY and lower_bound, count limit only via explicit Last N.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Rozhodnutí uživatele (2026-11-03): Live nemá implicitní limit počtu zpráv; jediný limit je ručně zadané 'Last: N'. AC #2 upraveno podle toho.

Recenze plánu i diffu: agent_review OK. Měření na reálném objemu (13 841 zpráv): replay 0,69 s, 71 publikací, klon grafu 12-14 ms, peak RSS 70-83 MB; UI vlákno graf přebírá přesunem bez klonu.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Zprávy se načítají jen follower replayem od --after-id 0 (celá aktuální DB, bez --last 500 a bez 16 MiB limitu výstupu); hrany drží celou historii (zrušen MAX_EDGE_HISTORY a příznak N+/lower_bound); počet omezuje jen explicitní Last N, i v Live; try_get_latest předává graf přesunem. Ověřeno testy na 600 zprávách (Live bez limitu, pevný rozsah s přesným počtem, Last N), testem replay/db_reset bez --last, fmt/clippy/146+17 testů.
<!-- SECTION:FINAL_SUMMARY:END -->

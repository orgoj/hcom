---
id: TASK-77.36
title: Načítání zpráv podle nastaveného rozsahu bez limitu počtu (limit jen pro Live)
status: To Do
assignee: []
created_date: '2026-11-03 15:30'
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
- [ ] #1 Při nastavení časového rozsahu (např. 'all' nebo interval od-do) se načtou a zobrazí všechny zprávy spadající do daného intervalu bez omezení na výchozí snapshot limit 500 zpráv
- [ ] #2 Limit na počet zpráv se uplatňuje výhradně v režimu Live (nebo při explicitním zadání filtru 'Last: N')
- [ ] #3 Všechny komunikační hrany v daném intervalu mají přesné počty a historii odpovídající všem uloženým zprávám v databázi
<!-- AC:END -->

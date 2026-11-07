---
id: TASK-77.82
title: 'Dashboard: dialog Resume/Fork s pojmenovaným tlačítkem a náhledem příkazu'
status: To Do
assignee: []
created_date: '2026-11-07T15:58:00Z'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: medium
type: enhancement
ordinal: 177000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
UX review 2026-11-07 (F6, A8, Q4 potvrzeno). Dialogy Resume/Fork končí obecným OK, které spouští reálného agenta. Řešení: slabá nápověda pod titulkem (Resume: 'Continues this session in a new terminal.'; Fork: 'Starts a new session from this one; the original stays.'), primární tlačítko Resume / Fork místo OK (Enter jako dnes), read-only náhled příkazu slabým monospace pod polem prompt: hcom r <id> / hcom f <id>, --dir jen po změně, prompt jako "…"; živě se aktualizuje. Pořadí Tabu dir → Browse → prompt → primární → Cancel; Escape zruší. Při testu primární tlačítko nikdy nemačkat.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Primární tlačítko dialogu zní Resume (u Fork: Fork) a náhled ukazuje hcom r <id> / hcom f <id>; --dir … se objeví až po změně dir
- [ ] #2 Dialog ukazuje jednořádkovou nápovědu rozlišující Resume a Fork
<!-- AC:END -->

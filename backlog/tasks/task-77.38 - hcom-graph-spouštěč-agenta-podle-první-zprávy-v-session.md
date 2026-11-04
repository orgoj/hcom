---
id: TASK-77.38
title: 'hcom-graph: spouštěč agenta podle první zprávy v session'
status: To Do
assignee: []
created_date: '2026-11-04 23:32'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: medium
type: feature
ordinal: 129000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Strom spuštění v hcom-graph bere rodiče jen z parent_name v hcom list --json. hcom ho vyplňuje jen u Claude Task/Agent subagentů; agenti spuštění autostartem přes hcom send (včetně roaming) ani ostatní rodiče nemají, takže Hierarchy je jedna řada kořenů. Pravidlo uživatele: spouštěč se určí podle první zprávy v každé session agenta. Pokud session začala promptem nebo nemá žádnou zprávu, spustil ji uživatel. Pokud je první zpráva ta, která agenta autostartem přes send probudila, spustil ho odesílatel té zprávy. Souvisí s TASK-77.25 (layout Hierarchy).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Agent, jehož session začala autostartem přes hcom send, má ve stromu spuštění za rodiče odesílatele první zprávy v té session
- [ ] #2 Agent, jehož session začala promptem nebo v ní není žádná zpráva, nemá spawn rodiče (spustil ho uživatel) a je kořenem
- [ ] #3 Nová session téhož agenta určí spouštěče znovu podle své první zprávy
- [ ] #4 Claude subagenti dál přebírají rodiče z parent_name
- [ ] #5 Regresní testy pokrývají autostart přes send, prompt, session bez zprávy a novou session
<!-- AC:END -->

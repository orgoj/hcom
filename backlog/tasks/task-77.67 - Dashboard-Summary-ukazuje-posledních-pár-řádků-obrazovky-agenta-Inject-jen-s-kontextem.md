---
id: TASK-77.67
title: >-
  Dashboard: Summary ukazuje posledních pár řádků obrazovky agenta, Inject jen s
  kontextem
status: To Do
assignee: []
created_date: '2026-11-07T12:01:00Z'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: high
type: bug
ordinal: 162000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Uživatel: pravý panel Summary pořád nemá posledních několik řádků ze Screen; u pole Inject v Summary nevím, do čeho píšu. Teď Summary má jen Quick Actions, pole Inject (vždy s Enter) a volitelně Recent Transcript; obrazovka se načítá jen na záložce Screen. Otevřené rozhodnutí: pole Inject v Summary buď zůstane pod náhledem obrazovky (stejný Enter checkbox jako ve Screen), nebo se ze Summary odstraní a inject zůstane jen ve Screen.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Summary běžící instance ukazuje posledních několik řádků obrazovky (hcom term), obnovovaných jako záložka Screen
- [ ] #2 Instance bez obrazovky (stopped, bez PTY) ukáže místo náhledu krátkou poznámku nebo nic
- [ ] #3 Inject v Summary je buď přímo pod náhledem obrazovky, nebo je ze Summary odstraněný (podle rozhodnutí uživatele)
- [ ] #4 Headless test ověří náhled v Summary
<!-- AC:END -->

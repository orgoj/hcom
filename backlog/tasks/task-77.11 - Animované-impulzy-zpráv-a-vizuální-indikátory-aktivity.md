---
id: TASK-77.11
title: Animované impulzy zpráv a vizuální indikátory aktivity
status: To Do
assignee: []
created_date: 2026-11-01 14:54
updated_date: '2026-11-06T21:30:00Z'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: low
type: feature
ordinal: 88000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Vizuální oživení 2D dashboardu: animované světelné impulzy (packet pulses) pohybující se po komunikačních hranách při doručení nových zpráv, pulzující záře kolem aktivně pracujících agentů (provádějících nástroje/příkazy) a vizuální indikace chybových/blokovaných stavů.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Aktivně pracující agent má jemně pulzující záři / indikátor aktivity
- [ ] #2 Animace jsou plynulé, šetrné k CPU a nezpůsobují zbytečné překreslování při nečinnosti
- [ ] #3 Při příchodu nové zprávy za běhu (ne historie při startu) projede po viditelné hraně ve vrstvě Msgs od odesílatele k příjemci plynulý impulz za ~1 s
- [ ] #4 Zablokovaný agent (blocked) má navíc k barvě stavu varovný indikátor; jiný chybový stav hcom nemá
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Uživatel (2026-11-06): AC #3 jen blocked (hcom nemá stav chyba); impulz ~1 s jen pro nové živé zprávy. Otevřené: indikátor aktivních agentů — uživatel navrhuje šetrnější blikání změnou barvy místo plynulé záře.
<!-- SECTION:NOTES:END -->

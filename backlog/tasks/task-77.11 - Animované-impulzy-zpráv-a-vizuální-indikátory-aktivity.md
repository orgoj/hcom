---
id: TASK-77.11
title: Animované impulzy zpráv a vizuální indikátory aktivity
status: To Do
assignee: []
created_date: '2026-11-01 14:54'
updated_date: '2026-11-01 15:12'
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
- [ ] #1 Při příchodu nové zprávy projede po příslušné hraně plynulý světelný impulz (packet pulse)
- [ ] #2 Aktivně pracující agent má jemně pulzující záři / indikátor aktivity
- [ ] #3 Zablokovaný agent (blocked) nebo agent s chybou je zvýrazněn varovným indikátorem
- [ ] #4 Animace jsou plynulé, šetrné k CPU a nezpůsobují zbytečné překreslování při nečinnosti
<!-- AC:END -->

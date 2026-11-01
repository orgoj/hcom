---
id: TASK-77.7
title: 'Časový filtr a historie toku zpráv: time window a dynamická filtrace'
status: To Do
assignee: []
created_date: '2026-11-01 14:53'
labels: []
dependencies: []
parent_task_id: TASK-77
type: feature
ordinal: 85000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Přidání volby časového okna pro graf a historii zpráv (např. předvolby 15 min, 1 hodina, Dnes, Vše, případně posuvník). Plátno zobrazí pouze komunikační hrany a agregace spadající do zvoleného okna; v záhlaví je jasně indikováno období, ze kterého data pocházejí.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Uživatelské rozhraní v horní liště nabízí volbu časového horizontu (15 min, 1h, 6h, 24h, Vše)
- [ ] #2 Komunikační hrany a čítače zpráv se dynamicky filtrují podle zvoleného časového okna
- [ ] #3 Záhlaví nebo stavová lišta zobrazuje přesné časové rozmezí zobrazených událostí
- [ ] #4 Hrany neaktivní ve zvoleném období jsou skryty nebo ztlumeny
<!-- AC:END -->

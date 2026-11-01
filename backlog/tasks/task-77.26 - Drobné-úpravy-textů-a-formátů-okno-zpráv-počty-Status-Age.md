---
id: TASK-77.26
title: 'Drobné úpravy textů a formátů (okno zpráv, počty, Status Age)'
status: To Do
assignee: []
created_date: '2026-11-01 19:40'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: low
type: feature
ordinal: 104000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
UX review F10-F12/S9: '0 min'/'0 msgs' čtou jako nic, popisek end je nejasný, dvě různé celkové hodnoty (Agents 45 vs Celkem 53) a české popisky v anglickém UI, Status Age v sekundách, prázdná pole s popiskem.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Vlastní rozsah 0 se zobrazuje jako 'all' a Last 0 jako 'no limit'
- [ ] #2 Pravý popisek posuvníku je 'now' při zapnutém Live, jinak čas konce v UTC
- [ ] #3 Záhlaví ukazuje jeden srozumitelný počet v angličtině (např. 'Shown 21 / 53') a 'Agents' i total mají vysvětlení v tooltipu
- [ ] #4 Status Age v panelu agenta má stejný relativní formát jako sloupec since a prázdná pole se nevypisují
<!-- AC:END -->

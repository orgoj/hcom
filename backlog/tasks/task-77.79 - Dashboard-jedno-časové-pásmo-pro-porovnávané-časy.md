---
id: TASK-77.79
title: 'Dashboard: jedno časové pásmo pro porovnávané časy'
status: To Do
assignee: []
created_date: '2026-11-07T15:57:00Z'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: medium
type: enhancement
ordinal: 174000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
UX review 2026-11-07 (F3, A5; Q1 rozhodnuto: všechny časy na obrazovce v místním čase, UTC jen v tooltipu). Karty sessions ukazují místní čas, pole okna a Range UTC se sekundami; tatáž session vypadá jako dva různé časy. README dnes uvádí Range záměrně v UTC: změnit i README. Přesnost na minuty, když okno vzniklo ze session.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Karta session a okno/Range po jejím Show in graph ukazují tytéž hodiny ve stejném (místním) pásmu, na minuty
- [ ] #2 UTC je dostupné v tooltipu Range
- [ ] #3 README popisuje místní čas
<!-- AC:END -->

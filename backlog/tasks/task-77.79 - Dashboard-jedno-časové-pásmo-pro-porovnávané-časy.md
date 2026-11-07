---
id: TASK-77.79
title: 'Dashboard: jedno časové pásmo pro porovnávané časy'
status: Done
assignee:
  - '@hcom_dev__idea'
created_date: '2026-11-07T15:57:00Z'
updated_date: '2026-11-07T16:27:00Z'
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
- [x] #1 Karta session a okno/Range po jejím Show in graph ukazují tytéž hodiny ve stejném (místním) pásmu, na minuty
- [x] #2 UTC je dostupné v tooltipu Range
- [x] #3 README popisuje místní čas
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Window text field, Range header, slider tooltip and edge Last Active show local time (minutes; seconds only when not 0); Range tooltip shows UTC; text without an offset parses as local. Show in graph rounds the session to whole minutes (start down, end up) and the card shows the same minutes (session_minutes). README updated. Tests: test_task_77_79_*, test_format_window_range_uses_local_time, 77.73 adjusted; gates green; agent_review OK (tmp/knowledge/task-77.79-review.md).
<!-- SECTION:FINAL_SUMMARY:END -->

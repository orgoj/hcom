---
id: TASK-77.65
title: 'Dashboard: řádky tabulky obarvené podle stavu agenta'
status: To Do
assignee: []
created_date: '2026-11-07T12:00:00Z'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: high
type: bug
ordinal: 160000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Uživatel: v tabulce nemají řádky barvu podle stavu. hcom-gui barví text řádku: active zelená #1a7f37, blocked oranžová tučně #d97706, needs_user modrá #2563eb, error červená, stopped šedá #888888, disabled světle šedá přeškrtnutá. Dashboard má mít totéž (barvy z palety pro světlé i tmavé téma), výběrový pruh zůstane čitelný.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Text řádku má barvu podle stavu agenta jako v hcom-gui (active, blocked, needs_user, error, stopped, disabled)
- [ ] #2 Barvy fungují ve světlém i tmavém tématu a vybraný řádek zůstane čitelný
- [ ] #3 Headless test ověří barvu řádků podle stavu
<!-- AC:END -->

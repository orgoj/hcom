---
id: TASK-77.54
title: 'Dashboard: chyba v pravém panelu zůstává po přepnutí na jiného agenta'
status: To Do
assignee: []
created_date: '2026-11-07T08:10:00Z'
updated_date: '2026-11-07T08:48:00Z'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: high
type: bug
ordinal: 149000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Chyba zobrazená v pravém panelu u jednoho agenta zůstává viditelná i po výběru jiného agenta. Chyba patří k agentovi, u kterého vznikla (upřesnění uživatele: 'server' = agent).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Chyba v pravém panelu se zobrazuje jen u agenta, u kterého vznikla
- [ ] #2 Po výběru jiného agenta se zobrazí jeho stav (a případná chyba), ne chyba předchozího agenta
<!-- AC:END -->

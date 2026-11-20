---
id: TASK-77.62
title: 'Idle klasifikátor: zvážit úplné odstranění'
status: Done
assignee: []
created_date: '2026-11-07T12:00:00Z'
updated_date: '2026-11-20T17:23:00Z'
labels: []
dependencies: []
parent_task_id: TASK-77
type: enhancement
ordinal: 157000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Uživatel opravu idle třídy pravým klikem (idle_check.py --label) nepoužívá: agenti píší status (status_column, hcom agent list --status) a ten je spolehlivější. Proto se do dashboardu oprava idle třídy neportuje a idle klasifikátor (sloupec Idle v hcom-dashboard a hcom-gui, contrib/idle-classifier) se možná úplně odstraní. Před rozhodnutím ověřit s uživatelem a local_models, kdo klasifikátor ještě používá.
<!-- SECTION:DESCRIPTION:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-11-20 user decision (via herdr_dev__dashboard): not doing this. The classifier is not removed; it may later get a different LLM. TASK-130 made the dashboard Idle column opt-in (idle_classifier in dashboard.json) instead.
<!-- SECTION:NOTES:END -->

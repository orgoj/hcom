---
id: TASK-77.72
title: 'Dashboard Sessions: název sešny'
status: To Do
assignee: []
created_date: '2026-11-07T14:58:00Z'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: medium
type: feature
ordinal: 167000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Uživatel 2026-11-07 (návrh 1): řádek sešny nemá popis. Zdroj: Claude transcript řádky {type:custom-title,customTitle} (/rename, poslední vyhrává) a {type:ai-title,aiTitle}; Codex ~/.codex/session_index.jsonl {id, thread_name}. Ostatní tooly bez názvu.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Řádek sešny začíná názvem: u Claude poslední custom-title, jinak poslední ai-title; u Codex thread_name ze session_index.jsonl podle session id; bez názvu se řádek neukazuje
- [ ] #2 Transcript se čte znovu jen při změně jeho velikosti (cache podle cesty a velikosti)
- [ ] #3 Testy pokrývají výběr názvu (custom před ai, poslední vyhrává, codex index, chybějící název)
<!-- AC:END -->

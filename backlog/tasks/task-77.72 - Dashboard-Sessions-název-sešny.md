---
id: TASK-77.72
title: 'Dashboard Sessions: název sešny'
status: Done
assignee:
  - '@hcom_dev__idea'
created_date: '2026-11-07T14:58:00Z'
updated_date: '2026-11-07T15:09:00Z'
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
- [x] #1 Řádek sešny začíná názvem: u Claude poslední custom-title, jinak poslední ai-title; u Codex thread_name ze session_index.jsonl podle session id; bez názvu se řádek neukazuje
- [x] #2 Transcript se čte znovu jen při změně jeho velikosti (cache podle cesty a velikosti)
- [x] #3 Testy pokrývají výběr názvu (custom před ai, poslední vyhrává, codex index, chybějící název)
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Název sešny: Claude custom-title před ai-title (poslední vyhrává), Codex thread_name ze session_index.jsonl; cache titulků podle cesty a velikosti transcriptu v runneru. Test test_task_77_72_session_titles; review bez nálezů.
<!-- SECTION:FINAL_SUMMARY:END -->

---
id: TASK-77.83
title: 'Dashboard Sessions: hlavička vysvětlí vypnuté Resume u běžícího agenta'
status: Done
assignee:
  - '@hcom_dev__idea'
created_date: '2026-11-07T15:58:00Z'
updated_date: '2026-11-07T16:46:00Z'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: low
type: enhancement
ordinal: 178000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
UX review 2026-11-07 (F7, A7). U běžícího agenta jsou všechna Resume šedá a důvod je jen v tooltipu (tooltipy fungují, ověřil uživatel). Řešení: druhý řádek hlavičky Sessions 'agent is running — Resume is off (Fork works)'; tooltipy u tlačítek zůstávají.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 U běžícího agenta hlavička Sessions říká, že Resume je vypnuté, dokud běží; tlačítka Resume zůstávají vypnutá
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Sessions header of a running agent adds the line 'agent is running — Resume is off (Fork works)' (RESUME_OFF_TEXT); Resume buttons stay disabled with their tooltips. README updated. Test test_task_77_83_sessions_header_explains_resume_off (running vs stopped); gates green; small change, no review.
<!-- SECTION:FINAL_SUMMARY:END -->

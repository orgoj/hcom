---
id: TASK-77.108
title: 'Dashboard: Alt+X v hledání tabulky nesmí napsat x'
status: Done
assignee: []
created_date: '2026-11-13T08:03:00Z'
updated_date: '2026-11-13T08:04:00Z'
labels:
  - bug
dependencies: []
parent_task_id: TASK-77
ordinal: 212000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Alt+X ve fokusovaném vyhledávacím poli tabulky pole vymaže, ale ve stejném snímku do něj TextEdit vloží událost Text("x"), takže v poli zůstane x.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Alt+X ve fokusovaném poli hledání ho vyprázdní a nevloží žádný znak
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Alt+X v hledání tabulky zahodí Text události daného snímku, takže se do pole nic nevloží; regresní test v test_task_77_51.
<!-- SECTION:FINAL_SUMMARY:END -->

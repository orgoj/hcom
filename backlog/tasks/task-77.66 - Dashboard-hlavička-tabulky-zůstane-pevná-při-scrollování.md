---
id: TASK-77.66
title: 'Dashboard: hlavička tabulky zůstane pevná při scrollování'
status: Done
assignee:
  - '@hcom_dev__idea'
created_date: '2026-11-07T12:00:00Z'
updated_date: '2026-11-07T13:31:00Z'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: high
type: bug
ordinal: 161000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Uživatel: při scrollování tabulky odjede i hlavička; hlavička sloupců musí zůstat pevně nahoře a scrollovat se mají jen řádky.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Při svislém scrollování zůstává hlavička tabulky viditelná nahoře
- [x] #2 Vodorovné scrollování posouvá hlavičku spolu se sloupci
- [x] #3 Headless test ověří polohu hlavičky po scrollu
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The header row scrolled out of view is redrawn at the viewport top over an opaque panel_fill band at its column x positions (follows horizontal scroll), with clickable sort buttons; rows scrolled into view land below it. Verified by test_task_77_66_table_header_stays_on_top_while_scrolling; agent_review approved.
<!-- SECTION:FINAL_SUMMARY:END -->

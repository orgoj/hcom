---
id: TASK-77.64
title: 'Dashboard: záložka Log označí novou chybu'
status: Done
assignee:
  - '@hcom_dev__idea'
created_date: '2026-11-07T12:00:00Z'
updated_date: '2026-11-07T13:38:00Z'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: medium
type: feature
ordinal: 159000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Jako v hcom-gui: nový ERROR záznam v Logu, zatímco je otevřená jiná záložka detail panelu, označí záložku Log (červená tečka); otevření Logu značku smaže.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Nová chyba při jiné otevřené záložce označí záložku Log
- [x] #2 Otevření záložky Log značku smaže
- [x] #3 Výstup bez chyby značku nepřidá
- [x] #4 Headless test
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
render_detail_tabs checks log lines added since the last frame: an [ERROR] line while another tab is open sets log_error_mark, shown as a red ● after Log (F8); the open Log tab clears it; non-error output adds none. Verified by test_task_77_64_log_tab_marks_a_new_error; agent_review approved.
<!-- SECTION:FINAL_SUMMARY:END -->

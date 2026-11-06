---
id: TASK-77.42
title: 'hcom-dashboard: Events, Transcript a Screen bez limitu 32 KiB výstupu'
status: Done
assignee:
  - '@hcom_dev__idea'
created_date: '2026-11-06T19:04:00Z'
updated_date: '2026-11-06T19:08:00Z'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: high
type: bug
ordinal: 135000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Záložka Events (F7) hlásí 'Command hcom output exceeded maximum allowed size of 32768 bytes' a nic nezobrazí. Events, Transcript i Screen čtou výstup hcom s limitem 32 KiB (action.rs). Uživatel nechce limit: záložky mají načíst celý výstup; počet událostí a řádků určuje už dotaz (--last).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Events, Transcript a Screen načtou celý výstup hcom bez ohledu na velikost; chyba o překročení velikosti se u nich neobjeví
- [x] #2 Test ověří, že čtení bez limitu vrátí výstup větší než 32 KiB celý
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Events, Transcript and Screen read hcom output without the 32 KiB cap (NO_STDOUT_LIMIT). Review finding fixed: stdout is read in a thread so the 30 s command_timeout also stops a command that keeps writing; capped actions still fail on overflow. Tests test_task_77_42_no_stdout_limit_reads_large_output_in_full and test_task_77_42_timeout_stops_endless_output_without_limit. Review agent_review OK.
<!-- SECTION:FINAL_SUMMARY:END -->

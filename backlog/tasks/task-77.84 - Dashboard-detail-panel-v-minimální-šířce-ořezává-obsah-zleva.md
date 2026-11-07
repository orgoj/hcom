---
id: TASK-77.84
title: 'Dashboard: detail panel v minimální šířce ořezává obsah zleva'
status: Done
assignee:
  - '@hcom_dev__idea'
created_date: '2026-11-07T15:58:00Z'
updated_date: '2026-11-07T16:52:00Z'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: medium
type: bug
ordinal: 179000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
UX review 2026-11-07 (F8, A9). Při minimální šířce detail panelu (~205 px, app_tester v3-15/16) chybí začátky řádků (ty pilot na Cyrilu, :04 – running), tlačítko Resume, názvy záložek (Summary → ary). Při ~330 px je vše v pořádku. Očekávání: zalamování / ořez jen vpravo.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Detail panel v minimální šířce: první znak každého řádku Sessions, každý název záložky a tlačítko Resume jsou viditelné (zalomení nebo ořez jen vpravo)
- [x] #2 Headless test pokrývá minimální šířku panelu
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Root cause: the Sessions card button row (ui.horizontal, ~390 px) and an untruncated title made the content wider than the 260 px minimum panel, so it overflowed (seen cut on the left in app_tester v3-16). Buttons now horizontal_wrapped; title truncated left of the right-aligned state (full on hover). README updated. Test test_task_77_84_min_width_detail_panel_not_clipped_left (fails before, passes after); gates green; agent_review OK.
<!-- SECTION:FINAL_SUMMARY:END -->

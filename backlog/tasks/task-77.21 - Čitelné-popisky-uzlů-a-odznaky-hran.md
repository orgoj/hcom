---
id: TASK-77.21
title: Čitelné popisky uzlů a odznaky hran
status: Done
assignee:
  - '@hcom_dev__idea'
created_date: '2026-11-01 19:40'
updated_date: '2026-11-02 19:24'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: medium
type: feature
ordinal: 99000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
UX review F5/S5: popisky se v hustých oblastech překrývají, odznaky '1 msgs' se kupí, text je negramatický.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Popisek uzlu má podklad v barvě plátna a nepřekrývá prstenec vlastního uzlu
- [x] #2 Při překryvu dvou popisků se skryje popisek uzlu s menším počtem zpráv v okně; hovered, vybraný uzel a sousedé vybraného uzlu mají popisek vždy
- [x] #3 Odznak hrany se ukáže jen pro počet >= 2 nebo pro hranu vybraného či hovered uzlu
- [x] #4 Jednotné číslo zní '1 msg', množné 'N msgs', dolní mez 'N+ msgs'
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Labels below the node ring on a canvas-colored background (label_rect), overlap resolved by visible_labels (busier node wins; hovered, selected and its neighbours forced), edge badge only for count >= 2 or focus edges (show_edge_badge), '1 msg' singular; fit_view includes the label below. README updated. Verified with new model_tests (label_rect, visible_labels, show_edge_badge, 1 msg) and the adjusted fit_view test; plan and diff reviewed by agent_review.
<!-- SECTION:FINAL_SUMMARY:END -->

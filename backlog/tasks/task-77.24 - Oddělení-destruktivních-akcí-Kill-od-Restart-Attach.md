---
id: TASK-77.24
title: Oddělení destruktivních akcí Kill od Restart/Attach
status: Done
assignee:
  - '@hcom_dev__idea'
created_date: 2026-11-01 19:40
updated_date: '2026-11-06T20:44:00Z'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: medium
type: feature
ordinal: 102000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
UX review F9/S8: Kill stojí hned vedle Restart. Potvrzovací dialogy Kill i Kill group už existují.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 V toolbaru tvoří Kill a Kill group samostatnou skupinu na konci řádku, oddělenou separátorem od Restart a Attach
- [x] #2 Kill group je neaktivní, když jeho menu nemá na co působit
- [x] #3 V panelu agenta je Kill oddělený mezerou od Attach a Transcript a stojí poslední
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Toolbar: separator after Restart, Kill + Kill group at the row end; Kill group wrapped in add_enabled_ui (enabled iff Filtered or any catalog/CLI group has a running target). Panel: KILL_GAP space before Kill. Headless test test_kill_actions_are_separated_from_restart_attach_and_transcript.
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Toolbar: separator after Restart; Kill and Kill group form the last group; Kill group disabled (add_enabled_ui) when neither Filtered nor any catalog/CLI group has a running target. Agent panel: KILL_GAP (16 px) before Kill, which stays last. Verified with headless test test_kill_actions_are_separated_from_restart_attach_and_transcript (mutation-checked), fmt, clippy 0 warnings, tests 17+167 passed; review OK (agent_review).
<!-- SECTION:FINAL_SUMMARY:END -->

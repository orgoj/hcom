---
id: TASK-77.57
title: 'Dashboard: globální Alt+šipka doleva/doprava přepíná záložky pravého panelu'
status: Done
assignee:
  - '@hcom_dev__idea'
created_date: '2026-11-07T09:43:00Z'
updated_date: '2026-11-07T09:46:00Z'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: medium
type: feature
ordinal: 152000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Doplněk k TASK-77.53 (Alt+šipka nahoru/dolů volí agenta): Alt+šipka doleva/doprava přepne na předchozí/další záložku pravého panelu detailu (Summary, Report, Screen, Transcript, Events, Log, Config, Legend), ať je fokus kdekoli.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Alt+šipka doprava přepne na další záložku pravého panelu, Alt+šipka doleva na předchozí, ať je fokus kdekoli (i v textovém poli)
- [x] #2 Na první/poslední záložce se výběr dál neposune
- [x] #3 Přepnutí se chová jako klik na záložku (načte obsah, skrytý panel zobrazí)
- [x] #4 Šipky bez Alt se chovají jako dosud
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Alt+Left/Right (consume_key ALT na začátku snímku) volí předchozí/další záložku detailu přes select_detail_tab, na krajích stojí. Test test_task_77_57_alt_left_right_switch_detail_tabs, review agent_review OK.
<!-- SECTION:FINAL_SUMMARY:END -->

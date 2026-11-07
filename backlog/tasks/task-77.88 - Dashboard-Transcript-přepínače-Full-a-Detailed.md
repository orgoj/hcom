---
id: TASK-77.88
title: 'Dashboard Transcript: přepínače Full a Detailed'
status: Done
assignee:
  - '@hcom_dev__idea'
created_date: '2026-11-07T17:53:00Z'
updated_date: '2026-11-07T18:01:00Z'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: medium
type: feature
ordinal: 183000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Uživatel 2026-11-07: transcript končí poznámkou hcom 'Note: Conversation text truncated. Use --full for full text; use --detailed for tool outputs, file edits, and errors.' (src/transcript/shared.rs flag_hint). Chce tlačítka, aby si transcript načetl úplný přímo v dashboardu. Rozhodnutí: dva přepínače Full a Detailed v záložce Transcript; zapnutý přepínač znovu načte transcript s --full / --detailed (i transcript session přes --path), platí pro další agenty a ukládá se do dashboard-state.json. Dnes: build_transcript_argv (action.rs) volá 'hcom transcript <agent> --last N', SessionTranscript '--path <p> --last 100'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Záložka Transcript má přepínače Full a Detailed; zapnutí znovu načte transcript s --full resp. --detailed
- [x] #2 Platí i pro transcript session ze Sessions (--path)
- [x] #3 Volba platí pro dalšího vybraného agenta a přežije restart (dashboard-state.json)
- [x] #4 README popisuje přepínače
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Transcript tab has persistent toggles Full/Detailed (DashboardState transcript_full/transcript_detailed); a change saves state and reloads via on_selection_changed with TranscriptFlags, for hcom transcript <agent> and --path session transcripts. Verified with test_task_77_88_transcript_full_detailed_toggles (argv, UI clicks, re-dispatch, persistence, session transcript of the next agent); fmt, clippy 0 warnings, 234+17 tests; review OK by agent_review.
<!-- SECTION:FINAL_SUMMARY:END -->

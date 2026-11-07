---
id: TASK-77.88
title: 'Dashboard Transcript: přepínače Full a Detailed'
status: To Do
assignee: []
created_date: '2026-11-07T17:53:00Z'
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
- [ ] #1 Záložka Transcript má přepínače Full a Detailed; zapnutí znovu načte transcript s --full resp. --detailed
- [ ] #2 Platí i pro transcript session ze Sessions (--path)
- [ ] #3 Volba platí pro dalšího vybraného agenta a přežije restart (dashboard-state.json)
- [ ] #4 README popisuje přepínače
<!-- AC:END -->

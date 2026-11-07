---
id: TASK-77.91
title: 'Dashboard Summary: odstranit blok Recent Transcript'
status: Done
assignee:
  - '@hcom_dev__idea'
created_date: '2026-11-07T18:29:00Z'
updated_date: '2026-11-07T18:30:00Z'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: medium
type: feature
ordinal: 186000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Uživatel 2026-11-07 po TASK-77.89: blok Recent Transcript ze Summary odstranit (od 77.89 se transcript na Summary načítá vždy, takže blok byl u každého běžícího agenta). Na něm závisí tlačítko Quick Actions 'Transcript' v Summary a 'View Transcript' v kontextové nabídce grafu: načítaly transcript (--last 30) jen pro tento blok; nově otevřou záložku Transcript.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Summary neukazuje blok Recent Transcript
- [x] #2 Tlačítka Transcript (Summary) a View Transcript (kontextová nabídka) otevřou záložku Transcript vybraného agenta
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Removed the Summary Recent Transcript block; its only feeders, Quick Actions Transcript and graph context menu View Transcript, now open the Transcript tab (select_detail_tab). Verified with test_task_77_91_summary_has_no_transcript_block; fmt, clippy 0 warnings, 237+17 tests. Small change: no review (SOUL step 5).
<!-- SECTION:FINAL_SUMMARY:END -->

---
id: TASK-77.87
title: 'Dashboard: pamatovat normální velikost a polohu okna'
status: Done
assignee:
  - '@hcom_dev__idea'
created_date: '2026-11-07T17:27:00Z'
updated_date: '2026-11-07T17:36:00Z'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: medium
type: feature
ordinal: 182000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Uživatel 2026-11-07 po TASK-77.86: maximalizace se obnovuje, ale normální velikost se nevrací. Okno zavřené nemaximalizované se má příště otevřít ve stejné velikosti a poloze; přepnutí maximalizováno → normální má vrátit pevnou velikost a polohu. Dnes: state ukládá jen window_size (record_window při maximized=Some(false)), polohu ne; main.rs předává with_inner_size/with_maximized, bez with_position. Podezření: přechodový snímek (maximized=false, ještě velký inner_rect) přepíše window_size; live state měl window_size [1439,1032] při window_maximized=false.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Okno zavřené nemaximalizované se při dalším startu otevře ve stejné velikosti a poloze
- [x] #2 Okno spuštěné maximalizované se po zrušení maximalizace vrátí do uložené normální velikosti a polohy
- [x] #3 Přechod mezi maximalizovaným a normálním stavem nepřepíše uloženou normální velikost a polohu velikostí maximalizovaného okna
- [x] #4 Headless test pokrývá uložení, obnovení a přechodové snímky
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
agent_review OK (tmp/knowledge/task-77.87-review.md). Committed and deployed; AC #1 and #2 wait for the user's desktop check.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Normal window size and outer position are saved (window_pos) and restored at start (with_position); a normal geometry counts once it held for 0.5 s, so maximize transition frames no longer replace it. User confirmed both scenarios on the desktop (the first failed check ran a dashboard started before the deploy). Test test_task_77_87_normal_geometry_ignores_transition_frames; agent_review OK.
<!-- SECTION:FINAL_SUMMARY:END -->

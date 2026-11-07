---
id: TASK-77.80
title: 'Dashboard: prázdný stav při selhání načtení agentů'
status: Done
assignee:
  - '@hcom_dev__idea'
created_date: '2026-11-07T15:57:00Z'
updated_date: '2026-11-07T16:32:00Z'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: medium
type: enhancement
ordinal: 175000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
UX review 2026-11-07 (F4, A11; Q5 rozhodnuto: binárka hcom se nemění, jeden nedostupný import katalogu smí shodit fetch, řeší se jen v dashboardu). Při selhání fetch zůstane graf i tabulka prázdná (Shown 0 / N), důvod jen v Logu ([ERROR] Failed to fetch live/agent data ... ×N). Řešení: text v těle tabulky a ve středu plátna 'Agent data could not be loaded — see Log (F8)' v barvě chybového textu, jeden řádek; zmizí s dalším úspěšným fetchem.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Po dalším úspěšném fetchi text zmizí
- [x] #2 Při selhaném fetchi agentů ukazuje prázdné tělo tabulky i prázdné plátno text Agent data could not be loaded — see Log (F8) v barvě chybového textu
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
AC #1 upřesněno na prázdný stav (titul úkolu, spec F4 'empty-state text'): se zastaralými řádky z dřívějšího úspěšného fetche se text neukazuje, data zůstávají a důvod je v Logu.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
After a failed agent fetch (TableDataFailed) with an empty table / no visible nodes, the table body and canvas centre show 'Agent data could not be loaded — see Log (F8)' in error red; the next TableDataLoaded clears it (agent_fetch_failed). AC #2 clarified to the empty state. README updated. Test test_task_77_80_failed_fetch_shows_empty_state_text; gates green; agent_review OK.
<!-- SECTION:FINAL_SUMMARY:END -->

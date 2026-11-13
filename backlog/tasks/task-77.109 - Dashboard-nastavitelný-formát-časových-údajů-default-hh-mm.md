---
id: TASK-77.109
title: 'Dashboard: nastavitelný formát časových údajů (default hh:mm)'
status: Done
assignee: []
created_date: '2026-11-13T08:34:00Z'
updated_date: '2026-11-13T09:26:00Z'
labels:
  - feature
dependencies: []
parent_task_id: TASK-77
ordinal: 216000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Sloupce Since/Age (format_age v contrib/hcom-dashboard/src/table_data.rs) ukazují jen největší jednotku (1h, 2d), takže chybí minuty. Uživatel chce v ~/.hcom/dashboard.json volbu formátu časových údajů; default hh:mm (např. 1:23 místo 1h). Při specifikaci upřesnit: zda jde o trvání (Since/Age) i absolutní časy (Sessions, okno zpráv), formát nad 24 h (např. 2d 03:15 nebo 51:15) a tvar konfigurace (strftime-like šablona nebo výčet).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 dashboard.json má volbu formátu časových údajů, default hh:mm
- [x] #2 Sloupce Since a Age zobrazují minuty podle zvoleného formátu
- [x] #3 Formát je popsaný v dokumentaci dashboardu a pokrytý testy
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
time_format v dashboard.json (default hh:mm, tokeny d/hh/h/mm/ss, short = dřívější tvar) pro Since, Age, Runtime, Status Age a délku session; README a test test_task_77_109_duration_formats. Review agent_review OK.
<!-- SECTION:FINAL_SUMMARY:END -->

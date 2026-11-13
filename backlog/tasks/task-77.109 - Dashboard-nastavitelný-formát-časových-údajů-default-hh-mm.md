---
id: TASK-77.109
title: 'Dashboard: nastavitelný formát časových údajů (default hh:mm)'
status: To Do
assignee: []
created_date: '2026-11-13T08:34:00Z'
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
- [ ] #1 dashboard.json má volbu formátu časových údajů, default hh:mm
- [ ] #2 Sloupce Since a Age zobrazují minuty podle zvoleného formátu
- [ ] #3 Formát je popsaný v dokumentaci dashboardu a pokrytý testy
<!-- AC:END -->

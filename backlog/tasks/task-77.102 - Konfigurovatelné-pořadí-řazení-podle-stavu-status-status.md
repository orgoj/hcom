---
id: TASK-77.102
title: Konfigurovatelné pořadí řazení podle stavu (status:status)
status: Idea
assignee: []
created_date: '2026-11-09T21:02:00Z'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: medium
type: feature
ordinal: 199000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Požadavek uživatele: řazení tabulky hcom-dashboard podle stavu nemá být řetězcové (sort_rows v contrib/hcom-dashboard/src/table_data.rs porovnává get_cell_value), ale podle toho, na co má uživatel reagovat; pořadí definovatelné v konfiguraci (~/.hcom/dashboard.json). Navržené výchozí pořadí: blocked, NEEDS_USER, FOLLOWUP (DONE_FOLLOWUP), DONE (jen běžící agenti), ostatní listening (např. WAITING), active, jiné stavy, všichni neaktivní (zastavení). Souvislost: status:status ukazuje živý stav mimo listening, jinak STATUS poslední odpovědi (TASK-77.101). Otevřené: kterých sloupců se to týká (status:status, status, oba), přesné názvy klíčů a tvar konfigurace, pořadí uvnitř skupiny, směr řazení po kliknutí na hlavičku. Needs analysis before implementation.
<!-- SECTION:DESCRIPTION:END -->

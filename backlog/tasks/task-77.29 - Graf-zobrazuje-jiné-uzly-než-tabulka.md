---
id: TASK-77.29
title: Graf zobrazuje jiné uzly než tabulka
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-11-02 21:01'
updated_date: '2026-11-03 11:17'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: high
type: bug
ordinal: 113000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Uživatel: tabulka ukazuje 53 agentů, v grafu je jich cca 24. Graf a tabulka jsou dvě rovnocenná zobrazení téže množiny agentů; při stejném filtru musí obě ukazovat stejné agenty.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Při každé kombinaci filtrů zobrazuje graf přesně ty agenty, které jsou řádky tabulky (stejný počet i jména)
- [x] #2 Počitadlo Zobrazeno/Celkem odpovídá oběma zobrazením
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Implement sync_graph_from_table_rows in app.rs to sync all catalog agents and instances into self.graph. 2. Align top status bar and table counts. 3. Add headless tests verifying graph and table node parity.
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Synchronized catalog agents and instances into AgentGraph, preserved spawn hierarchy and node fields, aligned header/table counts, added regression tests.
<!-- SECTION:FINAL_SUMMARY:END -->

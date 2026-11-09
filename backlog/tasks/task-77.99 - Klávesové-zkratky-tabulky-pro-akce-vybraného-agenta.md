---
id: TASK-77.99
title: Klávesové zkratky tabulky pro akce vybraného agenta
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-11-09T18:32:00Z'
updated_date: '2026-11-09T20:58:00Z'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: medium
type: feature
ordinal: 196000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Požadavek uživatele: v hcom-dashboard zkratky na vybraný řádek tabulky: s = Start..., Del = Kill (s potvrzením), m = Send..., r = Restart (s potvrzením), Ctrl+C = jméno agenta do schránky, Home/End a PageUp/PageDown v tabulce, zkratky v tooltipech tlačítek. Platí jen když klávesnici nemá textové pole a není otevřený dialog (jako open_with key). Při kolizi s open_with key z dashboard.json má přednost konfigurace.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 s otevře Start dialog pro vybraného zastaveného katalogového agenta, Del otevře potvrzení Kill pro vybraného běžícího, m otevře Send dialog, r otevře potvrzení Restart; jen když je tlačítko povolené
- [x] #2 Ctrl+C zkopíruje jméno vybraného agenta do schránky
- [x] #3 Home/End vyberou první/poslední řádek, PageUp/PageDown posunou výběr o stránku
- [x] #4 Zkratky nefungují s fokusem v textovém poli ani s otevřeným dialogem; open_with key se stejným znakem má přednost
- [x] #5 Tooltipy tlačítek Start/Send/Restart/Kill uvádějí zkratku; README popisuje zkratky
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented in contrib/hcom-dashboard/src/app.rs (toolbar shortcuts s/m/r/Delete/Ctrl+C, table Home/End/PageUp/PageDown), README, tests test_task_77_99_*. Dashboard fmt, clippy, 251 tests green. Ctrl+C yields to a mouse text selection (LabelSelectionState), no test for that path. Uncommitted diff sent to agent_review.

agent_review approved without findings. Done after the user's check in the app.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Implemented, tested and accepted by the user in the running dashboard.
<!-- SECTION:FINAL_SUMMARY:END -->

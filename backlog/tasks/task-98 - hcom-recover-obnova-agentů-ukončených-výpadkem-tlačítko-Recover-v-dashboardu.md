---
id: TASK-98
title: >-
  hcom recover: obnova agentů ukončených výpadkem + tlačítko Recover v
  dashboardu
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-11-11T09:02:00Z'
updated_date: '2026-11-11T09:27:00Z'
labels: []
dependencies: []
priority: medium
type: feature
ordinal: 201000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Po výpadku elektřiny chce uživatel jedním příkazem obnovit (hcom r) agenty, které výpadek ukončil a neměli dokončenou práci. Rozhodnutí uživatele: subcommand hcom (žádný contrib skript) a tlačítko v hcom-dashboard. Binárka zůstává obecná: kandidát = poslední stop agenta má reason process_exit (by:system, proces zmizel bez hooku), agent teď neběží, lokální, v posledních N hodinách. Řádek STATUS (DONE/NEEDS_USER) je uživatelova konvence: čte ho jen dashboard z transkriptu a předem zaškrtne agenty bez DONE.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 hcom recover vypíše kandidáty (jméno, nástroj, čas stopu, poslední stav hcom, adresář); --json vrací pole objektů
- [x] #2 Agent zabitý (killed), normálně ukončený, nyní běžící nebo vzdálený se mezi kandidáty neobjeví
- [x] #3 hcom recover <jméno>... obnoví uvedené agenty přes hcom r; hcom recover --all obnoví všechny kandidáty
- [x] #4 Nápověda (hcom recover --help, přehled hcom --help), README, docs a skill popisují příkaz
- [x] #5 hcom-dashboard: tlačítko Recover... otevře seznam kandidátů se zaškrtávátky; předem zaškrtnuté jsou ty, jejichž status:status (status column) není DONE ani DONE_FOLLOWUP; potvrzení spustí hcom recover se zaškrtnutými jmény
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
STATUS se nečte z transkriptu: hcom už ukládá [[status_column]] hodnoty (kv status_columns:<instance>), hcom recover --json je vrací v poli status_columns a dashboard z nich předvybírá. Agy normálně končí jako process_exit, proto mezi kandidáty bývají i staré antigravity sešny; dashboard je předem nezaškrtne, když mají DONE.

Review agent_review: schváleno; přidán ScrollArea v dialogu, --json při resume zamítnuto (TASK-99).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Nový subcommand hcom recover (výpis, <jména>, --all, --json se status_columns, --hours) a tlačítko Recover... v hcom-dashboard s předvýběrem podle status:status. Ověřeno unit testem candidates, cli_smoke recover_lists_no_candidates_on_a_clean_install, dashboard testy test_task_98_*, živý výpis hcom recover --json.
<!-- SECTION:FINAL_SUMMARY:END -->

---
id: TASK-89
title: >-
  approval-review: dippy_dev smí číst obrazovku agenta a snímek nese řádek z
  Dippy auditu
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-11-04 08:03'
updated_date: '2026-11-04 08:05'
labels: []
dependencies: []
priority: medium
type: feature
ordinal: 126000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Uživatel schválil požadavky dippy_dev: (1) zpráva zakazuje jen injektování a sahání na terminál agenta; čtení přes hcom term je povolené, když snímek nestačí. (2) Snímek končí posledním ask řádkem z dippy audit pro daného agenta (rozhraní od dippy_dev: --decision ask --context-flag =<instance>, záloha --cwd <dir> --agent <cli>, jinak no audit match).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Zpráva povoluje čtení přes hcom term <agent> a zakazuje jen injektování a jiné sahání na terminál
- [x] #2 Snímek končí posledním ask řádkem agenta s přesnou shodou jména v context_flags (substring shoda jiného agenta se nepoužije)
- [x] #3 Bez shody v context_flags se použije poslední ask pro cwd a CLI agenta; bez shody nebo při chybě dippy audit snímek uvede no audit match nebo chybu a zpráva se přesto odešle
- [x] #4 Auditovaný příkaz je jen v souboru 0600, ne ve zprávě; testy to pokrývají
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
approval_review.py: zpráva povoluje čtení přes hcom term, zakazuje jen injektování/sahání; snímek končí nejnovějším (podle ts) dippy audit ask řádkem s přesnou shodou jména, záloha cwd+CLI, jinak no audit match/chyba. Review agent_review OK (nález s pořadím výstupu opraven). Ověřeno: python3 -I contrib/approval-review/test_approval_review.py (8 OK).
<!-- SECTION:FINAL_SUMMARY:END -->

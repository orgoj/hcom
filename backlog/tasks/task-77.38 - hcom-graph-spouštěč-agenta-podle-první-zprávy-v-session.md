---
id: TASK-77.38
title: 'hcom-dashboard: spouštěč agenta podle první zprávy v session'
status: Done
assignee:
  - '@hcom_dev__idea'
created_date: 2026-11-04 23:32
updated_date: '2026-11-06T19:30:00Z'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: medium
type: feature
ordinal: 129000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Strom spuštění v hcom-graph bere rodiče jen z parent_name v hcom list --json. hcom ho vyplňuje jen u Claude Task/Agent subagentů; agenti spuštění autostartem přes hcom send (včetně roaming) ani ostatní rodiče nemají, takže Hierarchy je jedna řada kořenů. Pravidlo uživatele: spouštěč se určí podle první zprávy v každé session agenta. Pokud session začala promptem nebo nemá žádnou zprávu, spustil ji uživatel. Pokud je první zpráva ta, která agenta autostartem přes send probudila, spustil ho odesílatel té zprávy. Souvisí s TASK-77.25 (layout Hierarchy).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Agent spuštěný jiným agentem (autostart přes hcom send i hcom <cli> / hcom agent) má ve stromu spuštění za rodiče spouštěče z posledního life eventu batch_launched své session (pole by)
- [x] #2 Agent, jehož session spustil uživatel (by user/api/unknown/cli, nebo session bez batch_launched), nemá spawn rodiče a je kořenem
- [x] #3 Nová session (life event created) určí spouštěče znovu; spouštěč z předchozí session nepřetrvá
- [x] #4 Claude subagenti dál přebírají rodiče z parent_name, ten má přednost před spouštěčem
- [x] #5 Regresní testy pokrývají spuštění agentem, spuštění uživatelem, session bez batch_launched, novou session a přednost parent_name
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Rozhodnutí uživatele (2026-11-06): spouštěč = batch_launched.by místo pravidla první zprávy. Důvod: ze zpráv nelze poznat, zda session začala autostartem; hcom zapisuje spouštěče přímo (u autostartu přes send je by odesílatel, ověřeno na wdt_traffic: batch_launched by wdt_main, první zpráva od wdt_main). created.by se nepoužívá: loguje ho proces spouštěče z vlastního HCOM_LAUNCHED_BY, takže u wdt_traffic ukazuje cca_manager místo wdt_main. Handoff relaunch zapisuje batch_launched by user, takže agent po handoffu je kořen.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Rodič ve stromu spuštění = batch_launched.by poslední launche v aktuální session (reset na created), user/api/unknown/cli/system a self = kořen; parent_name subagentů má přednost. compute_event_stats + apply_event_stats (TableRow.launcher), sync_graph_from_table_rows bere launcher jako autoritativní po načtení stats a volá se i po StatsLoaded. Ověřeno: 2 nové regresní testy, fmt, clippy 0, 17+166 testů; review agent_review OK (nález o pořadí created/batch_launched zamítnut, TASK-77.43).
<!-- SECTION:FINAL_SUMMARY:END -->

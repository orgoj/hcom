---
id: TASK-77.4
title: >-
  Integrace funkcionality hcom-gui (tabulka agentů, panely
  Screen/Transcript/Log) jako náhrada Python verze
status: To Do
assignee: []
created_date: '2026-11-01 11:25'
updated_date: '2026-11-01 15:12'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: high
type: feature
ordinal: 82000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Rozšíření Rust GUI aplikace o plnohodnotnou náhradu stávajícího pythonového Tkinter hcom-gui. Aplikace bude kombinovat horní 2D grafový dashboard s dolním/bočním panelem obsahujícím tabulku agentů, filtraci, záložky Screen (hcom term), Transcript, Events, Log a ovládací panely (start, restart, inject, kill).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Tabulka instancí a katalogových agentů s řazením, filtry a zobrazením stavových sloupců
- [ ] #2 Spodní/postranní panel se záložkami pro Screen (interaktivní PTY/term view), Transcript, Events a Log
- [ ] #3 Toolbar a akční tlačítka (Start s parametry modelu, Restart, Kill, Inject prompt/klávesy)
- [ ] #4 Plná parita se stávajícím contrib/hcom-gui v modernější a rychlejší nativní Rust binárce
<!-- AC:END -->

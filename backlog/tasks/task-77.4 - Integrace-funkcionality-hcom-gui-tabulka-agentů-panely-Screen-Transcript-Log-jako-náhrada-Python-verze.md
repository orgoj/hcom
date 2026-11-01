---
id: TASK-77.4
title: >-
  Integrace funkcionality hcom-gui (tabulka agentů, panely
  Screen/Transcript/Log) jako náhrada Python verze
status: In Progress
assignee:
  - '@hcom_dev'
created_date: '2026-11-01 11:25'
updated_date: '2026-11-01 15:20'
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

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Konfigurační a stavový modul (config.rs): Načítání ~/.hcom/graph.json s fallbackem na ~/.hcom/gui.json; načítání ~/.hcom/graph-state.json s fallbackem na ~/.hcom/gui-state.json; načtení seznamu sloupců (columns), jejich šířek, řazení a filtrů; načtení screen_bar tlačítek, open_with akcí a report polí.
2. Datový model tabulky agentů (table_data.rs): Podpora všech sloupců hcom-gui (agent, instance, roaming, cli, context, quota, age, since, status, idle, status:*, detail, unread, dir, branch, git, where, starts, runtime, handoffs, msgs, stops, failed); agregace dat z hcom list/agent list/events.
3. UI tabulky a režimy zobrazení (table.rs / app.rs): Split view (graf nahoře, tabulka/panely dole), pouze graf, pouze tabulka; řazení kliknutím na hlavičku, obousměrná synchronizace výběru s 2D grafem, textové vyhledávání a filtry (running, blocked, hide disabled, hide ad-hoc).
4. Postranní panel – Report zobrazení: Výpis polí přesně podle konfigurace 'report' z gui.json/graph.json ve stylu klíč-hodnota.
5. Spodní záložky detailů: Screen tab s hcom term zobrazením a screen_bar klávesovou lištou (Inject, Enter, šipky, Esc, Tab, Ctrl+C/D); Transcript tab; Events tab; Log tab.
6. Toolbar a akční tlačítka: Start dialog (katalog, model, effort, prompt, resume); Restart tlačítko; Kill a Kill group (Filtered, CLI, Group).
7. Testy: Unit testy pro fallback konfigurace, parsování stavu, agregaci všech sloupců tabulky, filtrování a řazení.
<!-- SECTION:PLAN:END -->

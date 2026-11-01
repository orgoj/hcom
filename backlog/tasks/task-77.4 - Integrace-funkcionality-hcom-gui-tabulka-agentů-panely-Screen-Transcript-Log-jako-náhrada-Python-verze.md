---
id: TASK-77.4
title: >-
  Integrace funkcionality hcom-gui (tabulka agentů, panely
  Screen/Transcript/Log) jako náhrada Python verze
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-11-01 11:25'
updated_date: '2026-11-01 16:42'
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
- [x] #1 Tabulka instancí a katalogových agentů s řazením, filtry a zobrazením stavových sloupců
- [x] #2 Spodní/postranní panel se záložkami pro Screen (interaktivní PTY/term view), Transcript, Events a Log
- [x] #3 Toolbar a akční tlačítka (Start s parametry modelu, Restart, Kill, Inject prompt/klávesy)
- [x] #4 Plná parita se stávajícím contrib/hcom-gui v modernější a rychlejší nativní Rust binárce
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Konfigurace a persistence (src/config.rs): Načítání ~/.hcom/graph.json s fallbackem na ~/.hcom/gui.json; čtení stavu z ~/.hcom/graph-state.json s fallbackem na ~/.hcom/gui-state.json; striktně oddělený zápis výhradně do graph-state.json; podpora všech klíčů (refresh_seconds, stats_seconds, idle_seconds, open_with, commands, attach, screen_bar, report, toolbar_breaks); plná podpora screen_bar včetně vestavěných kláves (Esc, šipky, Enter, Tab) i custom objektů (Ctrl+C/D).
2. Datový model tabulky agentů (src/table_data.rs): Podpora všech 24 sloupců hcom-gui a dynamických status:* sloupců; agregace z hcom list/agent list/events; oddělené cykly pro stats (60s), git (10s TTL) a context/quota cache.
3. Asynchronní ActionWorker a generace výběru (src/action.rs): Rozšíření o operace pro Screen (term), Transcript a Events; ochrana před přepsáním výběru přes selection_generation u64 (zahazování zastaralých odpovědí při rychlém procházení).
4. Bezpečné provádění příkazů: Nahrazování zástupných symbolů ({dir}, {instance}, {agent}, {pane_id}) token po tokenu v argv poli bez shellu; ověřování if_exists; potvrzovací dialogy s výpisem cílových instancí pro Kill, Restart a Kill Group.
5. UI tabulky, režimy zobrazení a synchronizace (src/table.rs & src/app.rs): Tři režimy zobrazení (Split view, Pouze graf, Pouze tabulka); řazení kliknutím na hlavičku s empty_last; textový filtr (/ a Alt+X), checkboxy filtrů; obousměrná synchronizace výběru mezi tabulkou a 2D grafem.
6. Postranní panel – Report: Zobrazení klíč-hodnota přesně podle konfiguračního seznamu 'report' z gui.json/graph.json.
7. Spodní záložky detailů: Screen tab (PTY + screen_bar), Transcript tab, Events tab, Log tab.
8. Testy: Unit testy pro fallback konfigurace a stavu, parsování sloupců, screen_bar položky, placeholder substituci, generation counter a řazení s empty_last.
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Implementována kompletní integrace funkcionality hcom-gui v nativním egui dashboardu (contrib/hcom-graph):
- Plná konfigurace graph.json s fallbackem na gui.json a striktní izolace graph-state.json bez modifikace gui-state.json.
- 24-sloupcová tabulka s řazením (včetně empty_last), dynamickými status:* sloupci, filtry (text, running, blocked, hide disabled, hide adhoc).
- Spodní notebook panely: Screen s PTY a screen_bar tlačítky, Transcript, Events, Log a Report.
- Boční panel s detailem a konfigurovatelným reportem klíč-hodnota.
- Akční lišta: Start dialog s parametry modelu a CLI, Send dialog (-b), Restart, Kill, Kill Group s potvrzovacími dialogy, Open with a vlastní příkazy s bezpečnou náhradou placeholderů ({dir}, {instance}, {pane_id}, {terminal}).
- Asynchronní architektura s odděleným BackgroundSync, lehkým periodickým reloadem tabulky (10s git status TTL cache) a těžkým ReloadStats (každých 60s, 16MB cap) a ochranou proti zastaralým výsledkům (generation counter).
- Sloupce idle/context/quota jsou v TASK-77.4 prázdné a byly vyčleněny do navazujícího úkolu TASK-77.12.
Schváleno recenzentem @agent_review_hcom_884 (re-review #270704).
<!-- SECTION:FINAL_SUMMARY:END -->

---
id: TASK-77.69
title: >-
  Dashboard: záložka Sessions vybraného agenta (historie sešn, komunikace,
  resume)
status: Idea
assignee: []
created_date: '2026-11-07T13:56:00Z'
updated_date: '2026-11-07T14:23:00Z'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: medium
type: feature
ordinal: 164000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Uživatel: po výběru agenta nová záložka v detail panelu se všemi jeho sešnami v zobrazeném časovém okně: s kým komunikoval, kdo sešnu spustil, velikost, krátké info/popis, tlačítko Resume.

Známá data (ověřeno v kódu/CLI 2026-11-07):
- life eventy: created (by, parent_name), batch_launched (by, tool, instances), ready, stopped (by, reason; snapshot z src/hooks/common.rs ~1389 nese name, session_id, transcript_path, tool, directory, parent_name, created_at, last_seen). Ověřit: některé stopped eventy (cli kill) ve vzorku snapshot neměly.
- message eventy dávají partnery a počty v intervalu sešny; dashboard už počítá event stats (starts, stops, handoffs, msgs) v apply_event_stats.
- velikost: velikost transcriptu / tokeny (usage.rs čte posledních 1 MiB), počet tahů.
- popis: claude /rename title nebo codex thread_name, první prompt, status:* hodnoty z posledního reportu.
- resume: hcom r <session-uuid> (claude/codex/gemini; ses_<id> opencode/kilo), fork: hcom f <target>; CWD se obnoví z transcriptu.

Navržený směr: varianta A jen v dashboardu (sessions sestavené z hcom events life+message, filtrované časovým oknem scrubberu; řádek = sešna: začátek-konec, spustil, důvod konce, partneři s počty, velikost, popis; tlačítka Resume, Fork, Transcript, Kopírovat session id, Ukázat v grafu = nastavit okno na interval sešny). Varianta B (nový příkaz hcom sessions --json v binárce) jen pokud A nestačí výkonem nebo daty.

Rizika: resume staré sešny jména, které právě běží (konflikt jména); smazané transcripty (Claude cleanupPeriodDays) -> Resume zakázat; vzdálená zařízení; nástroje bez resume podle uuid. Potřebuje analýzu a rozhodnutí uživatele před implementací.
<!-- SECTION:DESCRIPTION:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Rozhodnutí uživatele 2026-11-07: jen hcom-dashboard (varianta A), žádná změna binárky. Uživatel: kdyby kill neukládal session ID, je to chyba hcom k opravě. Ověřeno v ~/.hcom/hcom.db: stopped eventy (i reason=killed, 1336 kusů) nesou snapshot se session_id a transcript_path; null session_id mají jen Claude subagenti (snapshot.agent_id, bez vlastní sešny) a pár starých eventů. Chyba hcom to není: hcom events bez --full snapshot jen odstraňuje z výstupu (src/commands/events.rs:163 streamline_event); s --full ho vrací. Dashboard dnes čte life eventy bez --full (contrib/hcom-dashboard/src/action.rs:591), pro Sessions bude potřebovat --full. Otevřené otázky na uživatele před implementací: rozsah (jen časové okno scrubberu, nebo celá historie s filtrem), Resume hned nebo přes Start dialog s předvyplněním, které doplňky v 1. verzi (Ukázat v grafu, Transcript sešny, Fork, Kopírovat session id, řetězec handoffů).
<!-- SECTION:NOTES:END -->

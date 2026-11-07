---
id: TASK-77.69
title: >-
  Dashboard: záložka Sessions vybraného agenta (historie sešn, komunikace,
  resume)
status: Done
assignee:
  - '@hcom_dev__idea'
created_date: '2026-11-07T13:56:00Z'
updated_date: '2026-11-07T14:42:00Z'
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

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Detail panel má záložku Sessions (za Transcript); pro vybraného agenta načte jeho life eventy (hcom events --type life --agent NAME --full) a message eventy (--participant NAME --full) a vypíše sešny (created..stopped, běžící bez konce), nejnovější nahoře
- [x] #2 Seznam ukazuje jen sešny, které zasahují do časového okna scrubberu (okno All = všechny); hlavička ukazuje počet v okně a celkem
- [x] #3 Řádek sešny ukazuje začátek, konec nebo running, délku, kdo spustil (agent z ready.by, user, nebo handoff po handoff eventu), důvod konce (reason/by), tool, adresář, session id (zkrácené), velikost transcriptu a partnery s počty odeslaných/přijatých zpráv v intervalu sešny
- [x] #4 Resume otevře Start dialog předvyplněný pro hcom r <session_id> (pole dir předvyplněné adresářem sešny a prompt; ostatní pole skrytá); OK spustí hcom r <session_id> s --dir jen při změně a --hcom-prompt jen když není prázdný
- [x] #5 Fork otevře týž dialog pro hcom f <session_id>
- [x] #6 Resume je zakázané, když agent právě běží, chybí session id, transcript na disku neexistuje nebo tool nepodporuje resume podle id (podporované: claude, codex, gemini, opencode, kilo); Fork je zakázaný bez session id, bez transcriptu nebo pro tool mimo claude, codex, opencode, kilo; zakázané tlačítko má tooltip s důvodem
- [x] #7 Transcript otevře záložku Transcript s transcriptem dané sešny (hcom transcript --path FILE --last 100), i u zastaveného agenta; volba záložky uživatelem nebo jiný agent ji vrátí na běžné chování
- [x] #8 Headless testy pokrývají sestavení sešn z eventů (spouštěč, handoff, důvod konce, partneři, běžící sešna), filtr oknem, stavy tlačítek a sestavení příkazů hcom r/f
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Modul src/sessions.rs: SessionInfo + parse_sessions(agent, life_jsonl, msg_jsonl, live) + overlaps(window). 2. ActionRequest::Sessions{agent,generation} / ActionResult::SessionsLoaded; worker spouští 2x hcom events + hcom list --json, stat transcriptu. 3. DETAIL_TABS + render Sessions záložky. 4. Start dialog režim session (Resume/Fork) a session_command(). 5. Transcript override cesty (ActionRequest::Transcript path). 6. Testy v tests/model_tests.rs, gates, review, commit, deploy.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Rozhodnutí uživatele 2026-11-07: jen hcom-dashboard (varianta A), žádná změna binárky. Uživatel: kdyby kill neukládal session ID, je to chyba hcom k opravě. Ověřeno v ~/.hcom/hcom.db: stopped eventy (i reason=killed, 1336 kusů) nesou snapshot se session_id a transcript_path; null session_id mají jen Claude subagenti (snapshot.agent_id, bez vlastní sešny) a pár starých eventů. Chyba hcom to není: hcom events bez --full snapshot jen odstraňuje z výstupu (src/commands/events.rs:163 streamline_event); s --full ho vrací. Dashboard dnes čte life eventy bez --full (contrib/hcom-dashboard/src/action.rs:591), pro Sessions bude potřebovat --full. Otevřené otázky na uživatele před implementací: rozsah (jen časové okno scrubberu, nebo celá historie s filtrem), Resume hned nebo přes Start dialog s předvyplněním, které doplňky v 1. verzi (Ukázat v grafu, Transcript sešny, Fork, Kopírovat session id, řetězec handoffů).

Specifikace 2026-11-07, odpovědi uživatele: rozsah = časové okno scrubberu; Resume = předvyplněný Start dialog; doplňky v 1. verzi = Fork a Transcript sešny (ne Ukázat v grafu, ne Kopírovat session id, ne řetězec handoffů). Mimo rozsah: popis z transcriptu (claude /rename title, codex thread_name, první prompt), počet tahů/tokeny, vzdálená zařízení (bez lokálního transcriptu je Resume/Fork zakázané), změny binárky. Fakta: ready.by = batch_launched.by stejné dávky (ověřeno na 98 dávkách), takže spouštěč sešny jde z life eventů agenta bez dotazu na cizí batch_launched; ~0.9 KB na stopped event se snapshotem, proto --full jen pro vybraného agenta, ne v ReloadStats. hcom r/f s --dir/--hcom-prompt ukazuje náhled jen uvnitř AI nástroje (src/commands/resume.rs:150), stejně jako hcom agent ze Start dialogu.

Implementace hotová, necommitnutá, čeká na review agent_review (diff tmp/task-77-69-review.diff). Gates zelené: fmt, clippy 0, 211 model_tests + 17 lib. Měření: hcom events life --agent hcom_dev --full 0,03 s / 376 KB, message --participant --full 0,18 s / 860 KB, takže reload při každém obnovení tabulky je levný.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Nová záložka Sessions v hcom-dashboard (src/sessions.rs, ActionRequest::Sessions/SessionTranscript, režim Start dialogu pro hcom r/f <session id>, přepsání Transcript záložky na soubor sešny). Ověřeno: 4 nové headless testy (test_task_77_69_*), fmt, clippy 0, 211 model_tests + 17 lib; review agent_review bez nálezů.
<!-- SECTION:FINAL_SUMMARY:END -->

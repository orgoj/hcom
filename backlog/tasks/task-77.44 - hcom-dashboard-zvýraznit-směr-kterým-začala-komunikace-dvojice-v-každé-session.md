---
id: TASK-77.44
title: >-
  hcom-dashboard: zvýraznit směr, kterým začala komunikace dvojice v každé
  session
status: Done
assignee:
  - '@hcom_dev__idea'
created_date: '2026-11-06T19:41:00Z'
updated_date: '2026-11-06T20:54:00Z'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: medium
type: feature
ordinal: 137000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Mezi dvěma agenty jsou dnes šipky zpráv v obou směrech (tloušťka podle počtu zpráv); ty zůstávají beze změny. Chybí hlavní šipka, která ukáže, kdo komunikaci zahájil: u agenta, kterého se všichni ptají (např. knowledge), nelze poznat, že začali tazatelé. Rozhodnutí uživatele (2026-11-06): komunikace dvojice A–B se dělí na úseky, nový úsek začíná novou session kohokoli z dvojice (life event created); iniciátor úseku = odesílatel první zprávy v úseku, která není odpovědí (reply_to, intent ack). Hlavní šipka je samostatná, vede od iniciátora; když úseky zahájily obě strany, má hroty na obou koncích. Mimo rozsah: tloušťka šipek zpráv a vypínání vrstev (samostatný task), strom spuštění (TASK-77.38).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Šipky zpráv dál ukazují oba směry, jednosměrná komunikace má jen jednu šipku
- [x] #2 Komunikace dvojice se dělí na úseky; nový úsek začne life eventem created kteréhokoli z dvojice
- [x] #3 Detail hrany a tooltip uvádějí iniciátora (u více úseků všechny s počtem)
- [x] #4 Iniciátor úseku je odesílatel první zprávy v úseku, která není odpovědí: zprávy s reply_to nebo s intent ack úsek nezahajují; úsek bez takové zprávy nemá iniciátora
- [x] #5 Regresní testy pokrývají jednosměrnou komunikaci, zahájení každou stranou, nový úsek po created jedné strany, odpověď (reply_to i ack) jako první zprávu úseku a oba zvýrazněné směry
- [x] #6 Mezi dvojicí, kde aspoň jeden úsek má iniciátora, je samostatná hlavní šipka (oddělená od šipek zpráv) směrem od iniciátora; zahájily-li úseky obě strany, má hlavní šipka hroty na obou koncích
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Data: MessageData.reply_to (any JSON) -> StoredMessage.is_reply; EventStatsEntry.session_starts = event ids of life created; app passes them to AgentGraph.set_session_starts on StatsLoaded. Model: AgentGraph.refresh_initiators (dirty flag, once per frame) splits each pair's merged history (by event id) at session starts of either side; segment initiator = first message that is neither reply nor ack. Canvas: separate translucent teal initiator arrow under the message edges (straight between reciprocal curves, curved beside a one-way edge), heads away from each initiating side. Edge detail 'Started by: X (n), Y (m)', new edge hover tooltip with the same line.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Uživatel (2026-11-06): odpovědi (reply_to, intent ack) konverzaci nezahajují; např. po restartu agenta mu druhý teprve odpovídá na dřívější dotaz, to není začátek konverzace.

Initiators use the whole retained history, independent of the message time window. Session boundaries compare event ids (life and message events share one id sequence). There was no edge tooltip before; AC #3 adds a pointer tooltip on message-edge hover.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Message events keep reply_to (StoredMessage.is_reply); life created event ids become session starts (EventStatsEntry.session_starts -> AgentGraph.set_session_starts). AgentGraph.refresh_initiators splits each pair's history at session starts of either side; a segment's initiator is its first non-reply, non-ack sender. Canvas draws a separate translucent initiator arrow under the message edges with heads away from each initiating side; edge detail and a new edge hover tooltip show 'Started by: X (n), ...'. Verified with 5 regression tests (task_77_44), fmt, clippy 0 warnings, tests 17+172 passed. Review: one finding (100000 life-event cap) rejected, TASK-77.47.
<!-- SECTION:FINAL_SUMMARY:END -->

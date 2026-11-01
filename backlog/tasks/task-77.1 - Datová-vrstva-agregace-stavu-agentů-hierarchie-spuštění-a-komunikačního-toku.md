---
id: TASK-77.1
title: 'Datová vrstva: agregace stavu agentů, hierarchie spuštění a komunikačního toku'
status: In Progress
assignee:
  - '@hcom_dev'
created_date: '2026-11-01 11:21'
updated_date: '2026-11-01 11:30'
labels: []
dependencies: []
parent_task_id: TASK-77
type: feature
ordinal: 79000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implementace načítání a aktualizace dat z hcom (čtení stavu z hcom list/events nebo SQLite). Poskytuje grafovou strukturu: uzly (instance, status, unread, age), hrany hierarchie (parent_name) a orientované hrany zpráv (from, delivered_to, count, last_ts).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Datový model drží uzly a dva typy hran (parent-child vazby a message-flow)
- [ ] #2 Podporuje periodický refresh nebo streamování událostí z hcom
- [ ] #3 Jednotkové testy ověřují správné sestavení grafu z testovacích dat
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Založit crate contrib/hcom-graph s vlastním Cargo.toml (petgraph, serde, serde_json, chrono).
2. Implementovat datové struktury v model.rs: AgentNode, SpawnEdge a MessageEdge (s počtem unikátních zpráv v aktuální epoše, timestampem a náhledem).
3. Implementovat AgentGraph s watermarkem last_event_id: události s id <= last_event_id se zahazují (idempotence bez neomezeného HashSetu).
4. Implementovat reset_events(): kompletní odstranění všech MessageEdge z grafu a vynulování last_event_id na 0 po db_reset.
5. Ingest kontrakt v source.rs: snapshot historie zpráv (hcom events) se před aplikací do grafu VŽDY seřadí vzestupně (newest-first reverse), streamování (events --follow --after-id <last_id>).
6. Deterministická db_reset sekvence: při detekci db_reset okamžitě zabít běžící child stream events --follow, zavolat reset_events(), provést a aplikovat seřazený initial snapshot a teprve poté spustit nový follow (zamezení race condition a přeskočení historie).
7. Unit testy v tests/model_tests.rs: sestavení grafu, akumulace zpráv, watermark dedup, řazení initial loadu a test db_reset synchronní resync sekvence.
<!-- SECTION:PLAN:END -->

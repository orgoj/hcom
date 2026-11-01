---
id: TASK-77.1
title: 'Datová vrstva: agregace stavu agentů, hierarchie spuštění a komunikačního toku'
status: In Progress
assignee:
  - '@hcom_dev'
created_date: '2026-11-01 11:21'
updated_date: '2026-11-01 11:28'
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
2. Implementovat datové struktury v model.rs: AgentNode (stav, unread, parent), SpawnEdge (hierarchie) a MessageEdge (směr, počet unikátních zpráv, timestamp, intent).
3. Implementovat AgentGraph s explicitním kurzorem (last_event_id), deduplikační sadou (processed_event_ids: HashSet<u64>) a idempotentním ingestem apply_message_event().
4. Definovat kontrakt zdroje dat v source.rs: initial load instancí (hcom list --json) a zpráv (hcom events --type message --full --all --last N), streaming s follow/reconnect (hcom events --follow --full --type message --after-id <last_id>) a obsluhu db_reset.
5. Unit testy v tests/model_tests.rs: sestavení grafu, akumulace cyklických zpráv, idempotentní aplikace překrývajících se dávek (ochrana proti zdvojení count a regresi timestampu) a db_reset.
<!-- SECTION:PLAN:END -->

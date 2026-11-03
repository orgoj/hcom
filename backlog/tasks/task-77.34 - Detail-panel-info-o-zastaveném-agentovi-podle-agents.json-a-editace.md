---
id: TASK-77.34
title: 'Detail panel: info o zastaveném agentovi podle agents.json a editace'
status: Done
assignee:
  - '@hcom_dev__idea'
created_date: '2026-11-03 13:04'
updated_date: '2026-11-03 14:02'
labels: []
dependencies: []
parent_task_id: TASK-77
type: feature
ordinal: 118000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Při výběru neaktivního/zastaveného agenta nebo agenta z katalogu zobrazit v pravém detail panelu jeho informace z agents.json (případně hcom agent json) s přípravou na budoucí editaci konfigurace.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Při výběru zastaveného agenta se v pravém panelu zobrazí informace o agentovi z agents.json
- [x] #2 Panel nabízí přehledné zobrazení konfigurace a atributů agenta z katalogu/json
- [x] #3 Připraveno rozhraní pro budoucí editaci parametrů agenta
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Při výběru zastaveného agenta z katalogu se v pravém detail panelu v nové záložce Config (a přes odkaz z prázdného Screen/Transcript/Events) zobrazují jeho atributy z agents.json (popis, CLI, model, reasoning, roaming, složka, skupiny, skills, instrukce, sloupce stavu) při striktním dodržení izolace katalogu. Panel obsahuje rozhraní s náhledem parametrů pro budoucí editaci. Pokryto integračními testy i aktualizovanou dokumentací.
<!-- SECTION:FINAL_SUMMARY:END -->

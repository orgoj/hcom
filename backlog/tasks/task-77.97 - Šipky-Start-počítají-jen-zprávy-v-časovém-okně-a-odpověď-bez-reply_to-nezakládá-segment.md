---
id: TASK-77.97
title: >-
  Šipky Start počítají jen zprávy v časovém okně a odpověď bez reply_to
  nezakládá segment
status: In Progress
assignee: []
created_date: '2026-11-09T06:44:00Z'
updated_date: '2026-11-09T06:59:00Z'
labels:
  - bug
dependencies: []
parent_task_id: TASK-77
priority: high
ordinal: 193000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Edge vrstva Start (initiator arrows) bere celou historii zpráv bez ohledu na časové okno a za iniciátora počítá i odpověď bez --reply-to (např. knowledge odpoví intent=inform po restartu). Hlášeno uživatelem přes hermes_dev__dashboard.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Iniciátor segmentu se počítá jen když jeho otevírací zpráva leží v časovém okně (span, end i last_n), stejně jako počty zpráv
- [x] #2 Zpráva od X k Y, která následuje po nezodpovězeném requestu od Y, segment neotevírá ani přes hranici restartu
- [x] #3 Detail hrany a tooltip ukazují iniciátory v okně
- [x] #4 Regresní testy v model_tests.rs
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Openery se hledají v celé historii (odpověď na request před oknem se pozná), počítají se jen ty v okně. Review agent_review OK; vědomá heuristika: první zpráva protistrany po dlouho nezodpovězeném requestu se bere jako odpověď. Čeká na kontrolu uživatelem.
<!-- SECTION:NOTES:END -->

---
id: TASK-77.5
title: >-
  Zabezpečení datové vrstvy: sanitizace řetězců, backoff streamu a ochrana proti
  zahlcení
status: In Progress
assignee:
  - '@hcom_dev'
created_date: '2026-11-01 11:57'
updated_date: '2026-11-01 11:59'
labels: []
dependencies: []
parent_task_id: TASK-77
type: feature
ordinal: 83000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Zapracování bezpečnostních nálezů z review agentem bin: 1. Důkladná sanitizace řetězců (odstranění control znaků, ANSI/OSC escapes, bidi overrides a zero-width znaků z preview, name a kontextů). 2. Exponenciální backoff při reconnectu streamu followeru. 3. Omezení maximální velikosti čtené řádky (1 MiB cap).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Sanitizační funkce odstraňuje řídicí znaky (C0/C1, ANSI/OSC), bidi přepisovače a zero-width znaky ze všech zobrazovaných řetězců
- [ ] #2 HcomSyncWorker implementuje exponenciální backoff při reconnectu streamu followeru
- [ ] #3 Čtení řádky ze streamu má limit maximální velikosti (ochrana proti OOM)
- [ ] #4 Jednotkové testy pokrývají sanitizaci i chování backoffu
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. model.rs: implementace plného ANSI/OSC stripingu (včetně payloadů) a odstranění C0/C1, bidi overrides a zero-width znaků pro všechna textová pole. Kolizní pravidlo pro identity: pokud sanitizace změní identitu (raw != sanitized), entita je neplatná a přeskočí se.
2. source.rs: čtení řádek s limitem 1 MiB a drainem zbytku řádky do dalšího \n (zamezení rozpadu na podřádky). Ne-blokující exponenciální backoff pro GUI pomocí deadline (Instant) pro EOF i I/O chyby.
3. tests/model_tests.rs: unit testy pro ANSI/OSC payload striping, kolizní pravidlo identit, čtení řádky s drainem nadlimitního vstupu a neblokující backoff.
<!-- SECTION:PLAN:END -->

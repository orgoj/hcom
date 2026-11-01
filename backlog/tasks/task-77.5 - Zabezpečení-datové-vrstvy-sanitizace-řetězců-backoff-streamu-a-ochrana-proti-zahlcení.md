---
id: TASK-77.5
title: >-
  Zabezpečení datové vrstvy: sanitizace řetězců, backoff streamu a ochrana proti
  zahlcení
status: In Progress
assignee:
  - '@hcom_dev'
created_date: '2026-11-01 11:57'
updated_date: '2026-11-01 12:00'
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
1. model.rs: ECMA-48 stavový automat/scanner pro 7-bit i C1 CSI a OSC sekvence (včetně subparametrů :, parametrů ?, intermediate bytes, final bytes @..~ a ST/BEL ukončení). Následné čištění C0/C1, bidi overrides a zero-width znaků. Kolizní pravidlo: pokud raw != sanitized pro jméno agenta, entita je neplatná a přeskočí se.
2. source.rs: čtení řádky s limitem 1 MiB a drainem zbytku řádky do dalšího \n. Ne-blokující exponenciální backoff s počátkem na 500 ms (další retry se zdvojnásobují až do 30 s) a sledováním deadline (Instant).
3. tests/model_tests.rs: unit testy pro ECMA-48 escape sekvence (subparametry, ~, C1 OSC/CSI, BEL/ST), kolizní pravidlo, capped read s drainem a časování neblokujícího backoffu.
<!-- SECTION:PLAN:END -->

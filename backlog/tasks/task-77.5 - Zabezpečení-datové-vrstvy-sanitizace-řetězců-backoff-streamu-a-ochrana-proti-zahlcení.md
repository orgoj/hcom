---
id: TASK-77.5
title: >-
  Zabezpečení datové vrstvy: sanitizace řetězců, backoff streamu a ochrana proti
  zahlcení
status: In Progress
assignee:
  - '@hcom_dev'
created_date: '2026-11-01 11:57'
updated_date: '2026-11-01 11:57'
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
1. model.rs: implementovat sanitize_display_string() pro odstranění C0/C1 control znaků, ANSI/OSC escapes, bidi overrides (U+202A-202E, U+2066-2069) a zero-width znaků ze všech zobrazovaných textů (preview, name, status_context, directory, tool).
2. source.rs: v SystemStreamHandle zastropovat čtení řádky na 1 MiB (take cap). V HcomSyncWorker přidat exponenciální backoff pro reconnecty streamu (0.5s -> 30s).
3. tests/model_tests.rs: unit testy pro sanitizaci (odstranění OSC/ANSI/bidi/zero-width) a pro exponenciální backoff workeru.
<!-- SECTION:PLAN:END -->

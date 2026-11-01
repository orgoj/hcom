---
id: TASK-77.5
title: >-
  Zabezpečení datové vrstvy: sanitizace řetězců, backoff streamu a ochrana proti
  zahlcení
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-11-01 11:57'
updated_date: '2026-11-01 12:14'
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
- [x] #1 Sanitizační funkce odstraňuje řídicí znaky (C0/C1, ANSI/OSC), bidi přepisovače a zero-width znaky ze všech zobrazovaných řetězců
- [x] #2 HcomSyncWorker implementuje exponenciální backoff při reconnectu streamu followeru
- [x] #3 Čtení řádky ze streamu má limit maximální velikosti (ochrana proti OOM)
- [x] #4 Jednotkové testy pokrývají sanitizaci i chování backoffu
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. model.rs: ECMA-48 parser s explicitním rozpoznáváním 7-bit (\x1b) i UTF-8 2-byte C1 kódů (\u{009B} = [0xC2, 0x9B], \u{009D} = [0xC2, 0x9D], \u{009C} = [0xC2, 0x9C]). Fail-closed chování: neukončené CSI/OSC sekvence se bezpečně zahazují od začátku sekvence až do konce řetězce. Kolizní pravidlo pro jména agentů: raw != sanitized způsobí přeskočení entity.
2. source.rs: read_line_capped s 1 MiB limitem a drainem do dalšího \n. Ne-blokující exponenciální backoff s 500 ms počáteční prodlevou a časovou deadlinou (Instant).
3. tests/model_tests.rs: testy pro UTF-8 C1 sekvence, fail-closed neukončené sekvence, subparametry, kolizní pravidlo, capped read s drainem a timing backoffu.
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Implementována ECMA-48 stavová sanitizace (7-bit i UTF-8 C1, CSI subparametry, fail-closed), striktní C0/C1 a bidi striping, raw identity validace, read_line_capped s drainem do \n, run_command_capped s 16 MiB limitem a souběžným drainem stderr proti deadlocku a SIGPIPE, neblokující exponenciální backoff (500ms..30s). Pokryto 16 testy, review OK.
<!-- SECTION:FINAL_SUMMARY:END -->

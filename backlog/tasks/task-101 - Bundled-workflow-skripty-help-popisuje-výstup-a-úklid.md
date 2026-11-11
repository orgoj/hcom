---
id: TASK-101
title: 'Bundled workflow skripty: --help popisuje výstup a úklid'
status: Done
assignee:
  - '@hcom_dev__analyze'
created_date: '2026-11-11T19:48:00Z'
updated_date: '2026-11-11T19:52:00Z'
labels: []
dependencies: []
priority: low
type: enhancement
ordinal: 205000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Agent se z hcom run <script> --help nedozví, kam jde výsledek workflow ani co po něm zůstane běžet. debate: VERDICT jako hcom zpráva volajícímu, průběh ve vlákně debate-<čas>-<pid>; --spawn nechá běžet PRO, CON i soudce (trap - EXIT, src/scripts/bundled/debate.sh:397). confess: verdikt soudce jako JSON zpráva volajícímu; calibrator, judge (a ve --fork confessor) zůstávají. fatcow: odpověď přichází hcom zprávou, live fatcow běží do hcom kill tag:fatcow.<path>. Uživatel rozhodl: seznam skriptů v bootstrapu nechat, jen zlepšit help.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 hcom run debate --help uvádí, kam jde verdikt, jak číst vlákno a jak ukončit agenty, kteří zůstanou běžet
- [x] #2 hcom run confess --help uvádí, kam jde verdikt a jak ukončit spuštěné agenty
- [x] #3 hcom run fatcow --help uvádí, kam přichází odpověď a co zůstává běžet v live a dead režimu
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Do --help skriptů debate, confess a fatcow přidány sekce Output a Cleanup (kam jde výsledek, jak číst vlákno, jak ukončit agenty, kteří zůstanou běžet); confess navíc vypíše Batch id pro hcom kill tag:. Ověřeno: hcom run <script> --help z target/debug, bash -n, fmt, clippy, cargo test --bin hcom (2912 ok).
<!-- SECTION:FINAL_SUMMARY:END -->

---
id: TASK-77.15
title: >-
  Nový layout: graf vlevo, záložky agenta vpravo, tabulka přes celou šířku dole
  (skrývatelná)
status: To Do
assignee: []
created_date: '2026-11-01 19:39'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: high
type: feature
ordinal: 93000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
UX review (F3): spodní inspect panel zabírá místo i prázdný, tabulka ukáže jen ~5 řádků. Rozhodnutí uživatele: dole přes celou šířku jen tabulka agentů (skrývatelná); nad ní vlevo graf, vpravo panel se všemi záložkami zvoleného agenta (Summary, Report, Screen, Transcript, Events, Log). Nahrazuje dnešní pravý panel detailu i spodní inspect panel. Mimo rozsah: persistence proporcí splitů (samostatný úkol).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Spodní oblast přes celou šířku okna obsahuje jen tabulku agentů
- [ ] #2 Tabulku lze skrýt a znovu zobrazit; skrytá tabulka uvolní místo grafu a panelu záložek
- [ ] #3 Nad tabulkou je vlevo graf a vpravo jeden panel se záložkami Summary, Report, Screen, Transcript, Events a Log pro zvoleného agenta
- [ ] #4 Dosavadní spodní inspect panel a samostatný pravý panel detailu už neexistují; F-klávesy záložek přepínají záložky pravého panelu
<!-- AC:END -->

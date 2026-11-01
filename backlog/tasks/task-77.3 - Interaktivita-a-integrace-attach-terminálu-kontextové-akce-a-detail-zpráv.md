---
id: TASK-77.3
title: 'Interaktivita a integrace: attach terminálu, kontextové akce a detail zpráv'
status: In Progress
assignee:
  - '@hcom_dev'
created_date: '2026-11-01 11:21'
updated_date: '2026-11-01 13:32'
labels: []
dependencies: []
parent_task_id: TASK-77
type: feature
ordinal: 81000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Uživatelská interakce s grafem: kliknutí na uzel aktivuje agenta v terminálu (attach), kliknutí na hranu otevře historii zpráv mezi vybranou dvojicí agentů, postranní panel s detaily a akcemi.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Kliknutí na uzel spustí attach/fokus agenta v terminálu
- [ ] #2 Kliknutí na hranu zobrazí zprávy vyměněné mezi těmito dvěma agenty
- [ ] #3 Boční panel nebo tooltip zobrazuje detail vybraného agenta (status detail, věk, poslední události)
- [ ] #4 Kontextové menu nabízí rychlé akce (inject, transcript, kill)
- [ ] #5 Kontextové/popup menu při pravém kliknutí na uzel (akce agenta) i na hranu (akce relace/detail zpráv)
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. ActionRunner abstrakce: Trait ActionRunner (SystemActionRunner pro produkci s hcom CLI a MockActionRunner pro testy) s metodami attach(agent), inject(agent, text), transcript(agent), kill(agent).
2. Detekce kliknutí a attach (AC #1): Rozlišení kliknutí od tažení (drag); kliknutí na uzel nastaví selected_node a zavolá runner.attach(agent).
3. Detekce hran a historie zpráv (AC #2): Hit-testing pro kvadratické Bezierovy křivky a přímé segmenty (vzdálenost k bodu myši < 8 px); kliknutí vybere hranu a zobrazí seznam zpráv a statistiku toku v postranním panelu.
4. Postranní panel a tooltipy (AC #3): Boční panel (egui::Panel::right) pro zobrazení detailu agenta (status, status detail, věk, nečtené zprávy, adresář, CLI nástroj, rodič) a detailu zpráv vybrané hrany. Tooltip při najetí myší na uzel/hranu.
5. Kontextová popup menu (AC #4, #5): Pravé kliknutí (secondary click) na uzel otevře kontextové menu s akcemi Attach, Inject prompt, Zobrazit transcript a Kill agenta. Pravé kliknutí na hranu otevře menu pro detail toku a zpráv.
6. Testy a verifikace: Unit testy pro hit-testing hran a uzlů, výběr, kontextová menu a odesílání akcí přes MockActionRunner.
<!-- SECTION:PLAN:END -->

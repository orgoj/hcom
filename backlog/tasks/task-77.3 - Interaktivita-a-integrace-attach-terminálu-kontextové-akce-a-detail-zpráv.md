---
id: TASK-77.3
title: 'Interaktivita a integrace: attach terminálu, kontextové akce a detail zpráv'
status: In Progress
assignee:
  - '@hcom_dev'
created_date: '2026-11-01 11:21'
updated_date: '2026-11-01 13:34'
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
1. Bounded & sanitizovaná historie zpráv: MessageEdge rozšířen o bounded VecDeque<StoredMessage> (max 50 zpráv), sanitizace textu přes sanitize_display_string, metoda get_conversation_history pro obousměrný chronologický výpis, reset při reset_database_state.
2. Asynchronní Background Action Worker: ActionWorker na pozadí s ActionRequest a ActionResult kanály; přímé předávání argumentů bez shellu (ochrana proti injection); bounded transcript (max 100 řádků / 32 KB); zobrazení indikátoru průběhu a chyb v panelu.
3. Destruktivní akce a chyby: Explicitní potvrzovací dialog před spuštěním kill (confirm_kill_target); zachycení selhání attach (ne-Herdr agent) se zobrazením chyby v UI panelu.
4. Hit testing a selekce: Detekce kliknutí vs. drag (prahová hodnota 3 px); hit testing Bezierových a přímých hran (práh 8 px); levý klik na uzel spustí asynchronní attach a vybere uzel; levý klik na hranu vybere hranu pro detail zpráv.
5. Postranní panel a kontextová menu: egui::Panel::right pro detail agenta (metadata, status context/age, inject pole, transcript výpis, kill tlačítko) a detail hrany (konverzační historie); popup menu na pravý klik pro uzel i hranu.
6. Testy: 7 unit testů pokrývajících bounded historii, obousměrný merge, DB reset, async dispatch workeru, selhání attach, potvrzení killu a hit-testing hran.
<!-- SECTION:PLAN:END -->

---
id: TASK-77.3
title: 'Interaktivita a integrace: attach terminálu, kontextové akce a detail zpráv'
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-11-01 11:21'
updated_date: '2026-11-01 14:26'
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
- [x] #1 Kliknutí na uzel spustí attach/fokus agenta v terminálu
- [x] #2 Kliknutí na hranu zobrazí zprávy vyměněné mezi těmito dvěma agenty
- [x] #3 Boční panel nebo tooltip zobrazuje detail vybraného agenta (status detail, věk, poslední události)
- [x] #4 Kontextové menu nabízí rychlé akce (inject, transcript, kill)
- [x] #5 Kontextové/popup menu při pravém kliknutí na uzel (akce agenta) i na hranu (akce relace/detail zpráv)
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Bounded & sanitizovaná historie zpráv: MessageEdge rozšířen o bounded VecDeque<StoredMessage> (max 50 zpráv na orientovanou hranu), sanitizace textu přes sanitize_display_string; get_conversation_history slučuje obousměrný tok a vrací striktně posledních 50 zpráv; reset při reset_database_state.
2. CLI dispatch a asynchronní ActionWorker: Ověřené CLI argumenty (agent attach <name>, term inject <name> <text> --enter, transcript <name> --last <N>, kill <name>); bounded fronta požadavků (sync_channel 16); bounded transcript (max 100 řádků / 32 KB); korektní drop/join při shutdownu.
3. Destruktivní akce a chyby: Potvrzovací dialog pro kill s ověřením stale targetu (zastavený/smazaný agent); zachycení selhání attach (ne-Herdr agent) se zobrazením chyby v UI panelu bez pádu.
4. Hit testing a selekce: Detekce kliknutí vs. drag (práh 3 px); hit testing Bezierových křivek a přímek (práh 8 px); levý klik na uzel spustí attach a vybere uzel; levý klik na hranu vybere hranu pro detail zpráv.
5. Postranní panel a kontextová menu: egui::Panel::right pro detail agenta (metadata, status context/age, inject pole, transcript výpis, kill) a detail hrany (chronologická historie); popup menu na pravý klik pro uzel i hranu.
6. Testy: 7 unit testů pokrývajících bounded historii, obousměrný limit 50, DB reset, async dispatch workeru, selhání attach, stale-target kill potvrzení a hit-testing hran.
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Implementována kompletní interaktivita a integrace: terminálový attach, historie zpráv mezi dvojicí agentů s obousměrným limitem a sanitizací, postranní panel detailů s inline injectem a transcriptem, asynchronní ActionWorker s pipe-safe souběžným drainem a spolehlivou cancelací při Dropu/ukončení, potvrzovací dialog pro kill s ověřením stale targetu a kontextová menu pro uzly i hrany. Pokryto 36 testy v contrib/hcom-graph.
<!-- SECTION:FINAL_SUMMARY:END -->

---
id: TASK-77.81
title: 'Dashboard Sessions: kompaktní karta s titulkem a odkazem handoffu'
status: Done
assignee:
  - '@hcom_dev__idea'
created_date: '2026-11-07T15:57:00Z'
updated_date: '2026-11-07T16:40:00Z'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: medium
type: enhancement
ordinal: 176000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
UX review 2026-11-07 (F5, A6, Q3). Karta má 7–8 řádků (~135 px, viditelné 2,7 karty). Nová karta: ř.1 titulek (nejsilnější), jinak slabé 'session <id8>', vpravo 'running · <živá délka>' / 'ended · <délka>'; ř.2 spojí start–konec (datum konce vynechat, je-li stejné), launched by, end reason, tool, velikost, id přes ' · '; řádky handoffu (continues from session … / handed off to session …) mezi ř.2 a adresářem; ř.3 adresář slabě s ~ místo home; ř.4 partners; ř.5 tlačítka. Handoff: hcom handoff loguje life action 'handoff' a dashboard ho páruje (TASK-77.75); handoff session je normální session s Resume/Fork. Klik na odkaz handoffu odscrolluje na kartu druhé session a krátce ji zvýrazní, je-li v seznamu. Data pro ověření: agenti s handoffem, např. hcom_dev__idea, contimex_main, bin.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Karta s titulkem i bez něj: první řádek je titulek nebo session <id8> a karta má při šířce panelu 600 px nejvýš 5 řádků + tlačítka (plus řádek na každý odkaz handoffu)
- [x] #2 Adresář je zobrazen s ~ místo domovského adresáře
- [x] #3 Klik na continues from / handed off to odscrolluje na kartu dané session v seznamu a zvýrazní ji
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Sessions card per UX spec §4.2: line 1 title or weak 'session <id8>' with 'running/ended · duration' right; line 2 time (end date only when different), launcher, end reason, tool, size, id (under a title); handoff lines are links when the other session is listed (scroll to its card, highlight 1.5 s, sessions_jump); directory with ~ (shorten_path). README updated. Test test_task_77_81_compact_card_and_handoff_link (600 px panel: 5 lines with handoff link, 4 without); gates green; agent_review OK.
<!-- SECTION:FINAL_SUMMARY:END -->

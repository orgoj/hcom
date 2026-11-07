---
id: TASK-77.81
title: 'Dashboard Sessions: kompaktní karta s titulkem a odkazem handoffu'
status: To Do
assignee: []
created_date: '2026-11-07T15:57:00Z'
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
- [ ] #1 Karta s titulkem i bez něj: první řádek je titulek nebo session <id8> a karta má při šířce panelu 600 px nejvýš 5 řádků + tlačítka (plus řádek na každý odkaz handoffu)
- [ ] #2 Adresář je zobrazen s ~ místo domovského adresáře
- [ ] #3 Klik na continues from / handed off to odscrolluje na kartu dané session v seznamu a zvýrazní ji
<!-- AC:END -->

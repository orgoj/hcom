---
id: TASK-77.14
title: Oprava pravého panelu detailu a stabilní rozdělení splitů
status: Done
assignee: []
created_date: '2026-11-01 17:47'
updated_date: '2026-11-01 17:56'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: high
type: bug
ordinal: 92000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Oprava zmizení pravého panelu s detailem (Detail Side Panel) a zajištění pevných a stabilních rozměrů všech splitů (sash). Pravý panel s detailem nesmí samovolně mizet a musí správně zobrazovat detaily zvoleného agenta vybraného z grafu i z tabulky (případně vybrané hrany), s možností explicitního zavření/otevření. Rozdělení mezi grafem a tabulkou (split horizontal/vertical) a spodní inspekční panel nesmí svévolně skákat a dynamicky měnit výšku/šířku při prázdných řádcích nebo překreslení, a uživatel musí mít možnost měnit poměr tažením oddělovače (sash).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Pravý panel detailu agenta/hrany zůstává zobrazen na pravé straně (nebo je toggleovatelný tlačítkem v toolbaru), nemizí samovolně při zrušení výběru na plátně a zobrazuje vybraného agenta jak z grafu, tak z tabulky.
- [x] #2 Pokud není vybrán žádný agent ani hrana, pravý panel ukazuje zřetelnou informaci 'No agent selected' a přehledové akce, nezpůsobuje zhroucení ani skákání layoutu.
- [x] #3 Výška spodního inspekčního panelu (Screen/Transcript/Events/Log/Report) je pevně ukotvena na uložené sash_pos a nemění se samovolně při změně filtrů, vyprázdnění tabulky nebo přepínání záložek.
- [x] #4 V módech Split H a Split V je mezi Grafem a Tabulkou k dispozici stabilní, interaktivní posuvník (sash/splitter) s perzistovanou pozicí rozdělení, který se dynamicky nezmenšuje.
<!-- AC:END -->

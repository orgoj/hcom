---
id: TASK-77.13
title: >-
  Oprava layoutu a filtrace grafu: propojení filtrů do plátna, oddělení tabulky
  a fixní spodní panel
status: Done
assignee: []
created_date: '2026-11-01 17:11'
updated_date: '2026-11-01 17:20'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: high
type: bug
ordinal: 91000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
1. Plátno 2D grafu musí respektovat filtry z horní lišty (Running only, Blocked only, Hide disabled, Hide ad-hoc, vyhledávací text) a zobrazovat pouze odpovídající uzly a hrany.
2. Tabulka agentů nepatří do spodního panelu. Hlavní úroveň přehledu agentů nabízí volbu: Pouze graf, Pouze tabulka, Split horizontálně (vedle sebe) nebo vertikálně (nad sebou).
3. Spodní panel je vyhrazen čistě pro inspekci zvoleného agenta (Screen, Transcript, Events, Log) a má pevnou výšku (sash_pos) nezávislou na počtu řádků v tabulce.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Plátno grafu dynamicky filtruje uzly a hrany podle horní lišty (Blocked only, Running only, text filtr)
- [x] #2 Tabulka agentů je v hlavní ploše na stejné úrovni jako graf s volbami zobrazení (Graf, Tabulka, Split H, Split V)
- [x] #3 Spodní panel obsahuje pouze inspekci agenta (Screen, Transcript, Events, Log) bez tabulky
- [x] #4 Spodní panel drží pevnou velikost zash_pos a nesmršťuje se při filtrování tabulky
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Opraven layout a filtrace grafu:
1. Plátno 2D grafu (CanvasView::show) nyní dynamicky přebírá a aplikuje visible_nodes – při aktivním filtru (Blocked only, Running only, textovém hledání atd.) vykresluje a propojuje pouze vyfiltrované uzly.
2. Tabulka agentů byla zcela odstraněna ze spodního inspekčního panelu.
3. Hlavní pracovní plocha nabízí 4 režimy: Graph (pouze graf), Table (pouze tabulka, zkratka F4), Split H (graf a tabulka vedle sebe), Split V (graf nad tabulkou).
4. Spodní inspekční panel (Screen, Transcript, Events, Log, Report) je pevně ukotven na spodní hraně okna, má nastavitelnou perzistovanou výšku (sash_pos) a jeho rozměr se nemění při filtrování tabulky.
<!-- SECTION:FINAL_SUMMARY:END -->

---
id: TASK-77.26
title: Drobné úpravy textů a formátů (okno zpráv, počty, Status Age)
status: Done
assignee:
  - '@hcom_dev__idea'
created_date: 2026-11-01 19:40
updated_date: '2026-11-06T21:26:00Z'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: low
type: feature
ordinal: 104000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
UX review F10-F12/S9: '0 min'/'0 msgs' čtou jako nic, popisek end je nejasný, dvě různé celkové hodnoty (Agents 45 vs Celkem 53) a české popisky v anglickém UI, Status Age v sekundách, prázdná pole s popiskem.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Záhlaví ukazuje jeden srozumitelný počet v angličtině (např. 'Shown 21 / 53') a 'Agents' i total mají vysvětlení v tooltipu
- [x] #2 Status Age v panelu agenta má stejný relativní formát jako sloupec since a prázdná pole se nevypisují
- [x] #3 Vlastní rozsah bez omezení se v textovém poli zobrazuje jako 'all' a Last 0 jako 'no limit'
- [x] #4 Tooltip posuvníku uvádí konec okna 'now' při zapnutém Live, jinak čas konce v UTC (stálý pravý popisek posuvník po přepracování okna nemá)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
AC #1 a #2 přepsány na současné UI (okno zpráv přepracováno po UX review: vlastní rozsah je textové pole, které už 'all' ukazuje; posuvník má jen tooltip). Rozsah se nemění.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Last 0 = 'no limit', tooltip posuvníku 'now' podle Live, záhlaví 'Shown N / M' s tooltipem (české Zobrazeno/Celkem pryč), Summary bez prázdných polí a Status Age ve formátu since (agent_meta_fields, format_age). Ověřeno test_task_77_26_last_limit_count_and_agent_fields_texts a upraveným test_task_77_29_table_and_graph_node_parity_and_visible_projection; fmt, clippy 0 warnings, cargo test 17+183. Review agent_review OK.
<!-- SECTION:FINAL_SUMMARY:END -->

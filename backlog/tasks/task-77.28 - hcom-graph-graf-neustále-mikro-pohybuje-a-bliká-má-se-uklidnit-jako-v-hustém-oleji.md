---
id: TASK-77.28
title: >-
  hcom-graph: graf neustále mikro-pohybuje a bliká, má se uklidnit jako v hustém
  oleji
status: To Do
assignee:
  - '@hcom_dev'
created_date: '2026-11-01 20:40'
updated_date: '2026-11-03 18:55'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: medium
type: bug
ordinal: 106000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Uživatel: graf vizuálně bliká, uzly mají pořád drobné mikropohyby. Chce celý graf zklidnit jako v hustém oleji (silné tlumení, po usazení klid).

Analýza (headless sonda, 14 uzlů se spawn stromem a zprávovými hranami, 60 s při 60 fps): layout v režimu Cluster ani Hierarchy nikdy neusne, takže se každý snímek simuluje a překresluje a auto-fit (TASK-77.18) při každém snímku přepočítá zoom/pan podle sub-pixelových posunů.
Příčiny:
1. Metrika pohybu je součet rychlostí. Uzel přitlačený silou do kontaktu má trvale nenulovou rychlost, ale kolize ho každý snímek vrátí; skutečný posun je ~0,01 px, metrika přesto 1,5–7,8 (práh 0,5).
2. Usnutí vyžaduje čistý kolizní průchod, ale v rovnováze s kontaktem integrace každý snímek znovu vytvoří sub-pixelový překryv (min. vzdálenost 78,5–79,8 px při 80 px), takže průchod není nikdy čistý.
3. damping 0.85 (zachová 85 % rychlosti za krok) je slabé tření; silnější tlumení znamená menší číslo (0.90/0.92 by bylo slabší).

Auto-fit sám chybu nemá: sleduje jen skutečný pohyb; když layout usne, pozice se nemění a fit dává stejný pohled.

Mimo rozsah: auto-fit, nezávislost tlumení na snímkové frekvenci, změny sil a rychlostních limitů, buzení layoutu při synchronizaci.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Kolizní průchod blokuje usnutí jen při překryvu větším než okraj 1 px obou uzlů (2 px); menší překryv se dál rozráží, ale kroužky uzlů se nepřekrývají
- [x] #2 Regresní test: realistický graf (spawn strom + zprávové hrany) usne v režimu Cluster i Hierarchy do 600 kroků po 1/60 s a po probuzení bez změny grafu znovu usne do 30 kroků; žádná dvojice uzlů pak není blíž než 78 px
- [x] #3 Testy, fmt a clippy hcom-graph projdou
- [x] #4 Metrika pohybu kroku layoutu je největší skutečný posun jednoho uzlu po kolizním průchodu, ne součet rychlostí
- [x] #5 Výchozí tlumení je silnější (damping 0.7) a práh usnutí epsilon_sleep 0.2 px na krok; uzel cestující za novým rodičem (existující testy přepojení v Cluster i Hierarchy) neusne uprostřed cesty
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. layout.rs step: uložit pozice před krokem, motion = největší skutečný posun uzlu po collide.
2. collide: collision_pass_clean=false jen při delta > 2*COLLISION_MARGIN (1 px).
3. Výchozí damping 0.7, epsilon_sleep 0.2 (měřeno: 0.6/1.0 se součtem uspalo uzel uprostřed přepojení; součet roste s počtem uzlů).
4. Regresní test v tests/model_tests.rs (nahradí měřicí sondu).
5. cargo fmt/clippy/test, review agent_review, commit, release build.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Reopened na žádost uživatele: graf se stále posunuje, někam utíká a nezastaví se. Příčina: periodické snímky ze synchronizace (sync.take_snapshot()) volají layout.wake_up() i bez strukturálních změn a auto-fit přepočítává pohled při každém subpixelovém pohybu.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Layout now sleeps: motion is the largest real per-node displacement after collision (contact pressure no longer counts), contact within the 2 px ring margin no longer keeps it awake, defaults damping 0.7 and epsilon_sleep 0.2. Verified with new regression test (Cluster and Hierarchy sleep within 600 steps, re-sleep within 30 after wake, no pair closer than 78 px), existing reparent tests, fmt, clippy, 106 tests; reviewed by agent_review.
<!-- SECTION:FINAL_SUMMARY:END -->

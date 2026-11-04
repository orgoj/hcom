---
id: TASK-77.28
title: >-
  hcom-graph: graf neustále mikro-pohybuje a bliká, má se uklidnit jako v hustém
  oleji
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-11-01 20:40'
updated_date: '2026-11-04 22:16'
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
- [x] #2 Testy, fmt a clippy hcom-graph projdou
- [x] #3 Metrika pohybu kroku layoutu je největší skutečný posun jednoho uzlu po kolizním průchodu, ne součet rychlostí
- [x] #4 Výchozí tlumení je silnější (damping 0.7) a práh usnutí epsilon_sleep 0.2 px na krok; uzel cestující za novým rodičem (existující testy přepojení v Cluster i Hierarchy) neusne uprostřed cesty
- [x] #5 Snímek synchronizace beze změny uzlů, spawn struktury a dvojic komunikujících agentů layout neprobudí; nová zpráva mezi agenty, kteří už spolu komunikovali, také ne
- [x] #6 Regresní test: realistický graf (spawn strom + zprávové hrany) usne v režimu Cluster i Hierarchy do 600 kroků po 1/60 s a po probuzení bez změny grafu znovu usne do 300 kroků; žádná dvojice uzlů pak není blíž než 78 px
- [x] #7 Strukturální změna (nový či odebraný uzel, změna spawn struktury, nová dvojice komunikujících agentů) a přepnutí režimu zvednou alpha na 1; pin a tažení aspoň na 0.3
- [x] #8 Regresní testy pokrývají usnutí podle alpha při vyčerpaném časovém rozpočtu, neprobouzení při nezměněném snímku a při zprávě na známé dvojici, probuzení při nové dvojici, novém uzlu a přepojení
- [x] #9 Všechny síly layoutu se násobí koeficientem alpha, který za krok klesá z 1 na 0.001 za 300 kroků; pod 0.001 běží jen kolizní průchod a layout usne po prvním úplném čistém průchodu začatém po vychladnutí, i když práh pohybu nesplnil nebo krok překročil časový rozpočet
- [x] #10 Regresní test: přechod alpha pod minimum uprostřed rozpracovaného kolizního průchodu neuspí layout podle starého čistého výsledku
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

Druhé kolo (po reopen): příčina je chybějící ochlazování. Síly působí stále plnou silou, usnutí závisí jen na prahu pohybu, který kolize proti pružinám a odpuzování nemusí nikdy splnit, a každý snímek synchronizace layout probudí. Oprava podle d3-force: alpha násobí všechny síly a za 300 kroků klesne z 1 na 0.001, pak layout usne natvrdo; probouzí jen strukturální změna (uzly, spawn struktura, nová dvojice komunikujících agentů, pin, tažení, režim).

Integrace zrychlena 4× (sub_dt = 2·dt místo dt/2): původní časová škála potřebovala tisíce kroků, takže by ochlazení zastavilo uzel cestující za novým rodičem v půlce. Strukturální změna ohřívá na 1, protože 0.3 na přesun nestačilo. Testy přepojení teď probouzejí layout přes sync_nodes (cesta aplikace) místo ručního wake_up. Flaky test realistického grafu: na nezměněném HEAD 1 selhání ze 40 běhů, s ochlazováním 0 ze 40; časové testy rozpočtu kroku selhávají i na HEAD (2–3 ze 40).

Review agent_review: nález (alpha stop podle zastaralého čistého kolizního průchodu) opraven restartem průchodu při přechodu pod alpha_min; regresní test bez opravy selže (překryv 0 px). Re-review OK.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Layout má ochlazování podle d3-force: alpha násobí všechny síly a za 300 kroků klesne z 1 na 0.001, pak běží jen kolize a layout usne po čistém průchodu. Probouzí jen strukturální změna v sync_nodes (alpha 1) nebo pin/tažení (alpha ≥ 0.3); aplikace už neprobouzí layout každým snímkem synchronizace. Integrace 4× rychlejší, aby přesun uzlu skončil během ochlazování. Ověřeno: 160 testů hcom-graph, fmt, clippy; layout testy 40× bez selhání dříve flaky testu; review agent_review OK.
<!-- SECTION:FINAL_SUMMARY:END -->

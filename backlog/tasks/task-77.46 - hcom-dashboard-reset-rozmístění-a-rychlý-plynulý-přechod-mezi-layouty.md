---
id: TASK-77.46
title: 'hcom-dashboard: reset rozmístění a rychlý plynulý přechod mezi layouty'
status: To Do
assignee: []
created_date: '2026-11-06T20:05:00Z'
updated_date: '2026-11-06T20:11:00Z'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: medium
type: feature
ordinal: 139000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Uživatel (2026-11-06): rozházení uzlů se mu nelíbí; osamělé uzly bez hran zůstávají daleko od ostatních; při přepnutí z Hierarchy (roztažené do řady) na Cluster vzdálené uzly dojíždějí nekonečně pomalu. Chce reset rozmístění, který graf přeskládá optimálněji, a progresivní rychlost: zrychlit, pak zpomalit a rychle se ustálit. Chvění se už zastavuje (TASK-77.28), to je v pořádku. Příčina pomalosti (layout.rs): každá síla se násobí alpha, která po set_mode klesá z 1 k alpha_min za ~900 kroků, a max_speed 270 px/s; vzdálený uzel nedojede, než síla vyhasne. Navržený směr: cílové rozmístění spočítat dopředu bez kreslení (simulace do ustálení, piny zůstávají) a uzly k němu animovat s ease-in-out (~0,5–1 s); totéž použít pro přepnutí režimu i pro tlačítko Reset rozmístění; osamělé uzly v cílovém rozmístění blízko ostatních (např. mřížka pod/vedle komponent). Otevřené: umístění tlačítka Reset (vedle Fit?), zda reset ruší piny, délka animace. Needs analysis before implementation.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Tlačítko Reset je v řádku s Fit hned vpravo od Fit; v režimu Manual je neaktivní
- [ ] #2 Reset v Hierarchy a Cluster přeskládá všechny nepřipnuté uzly do rozmístění aktuálního režimu spočítaného znovu od nuly (nezávisle na současných pozicích)
- [ ] #3 Připnuté uzly zůstávají při resetu i při přepnutí režimu na svém místě; kdo je chce přeskládat, musí je odepnout
- [ ] #4 Po resetu i po přepnutí mezi Hierarchy a Cluster dojedou uzly k cíli plynulou animací (zrychlení a zpomalení, bez skoku) za 3 až 5 s a potom je graf v klidu
- [ ] #5 Osamělé uzly bez hran končí v cílovém rozmístění u ostatních uzlů, ne daleko od nich
- [ ] #6 Regresní testy pokrývají cílové rozmístění bez závislosti na výchozích pozicích, nepohnuté připnuté uzly, délku a plynulost animace (bez skoku mezi snímky), neaktivní Reset v Manual a polohu tlačítka vedle Fit
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Výpočet cílového rozmístění: kopie stavu layoutu, nepřipnuté uzly na výchozí pozice nezávislé na současných, simulace bez kreslení do ustálení; osamělé uzly k ostatním. 2. Animace: interpolace z aktuálních pozic k cíli s ease-in-out po dobu ~4 s, piny se nehýbou, potom spánek layoutu. 3. Použití pro set_mode (Hierarchy↔Cluster) a pro tlačítko Reset vpravo od Fit (neaktivní v Manual). 4. Headless testy (kittest, widget_rect) a README. Riziko: výpočet cíle u velkého grafu nesmí zablokovat snímek (rozpočet; případně počítat ve více snímcích před startem animace).
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Uživatel (2026-11-06): Reset vpravo vedle Fit; piny zůstávají (pro přeskládání odepnout); animace 3–5 s, plynulá, bez skoku; v Manual je Reset neaktivní.
<!-- SECTION:NOTES:END -->

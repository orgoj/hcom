---
id: TASK-77.27
title: hcom-graph zamrzne po otevření kontextového menu a pravém kliku jinde v grafu
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-11-01 20:39'
updated_date: '2026-11-02 17:28'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: high
type: bug
ordinal: 105000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Analýza odhalila příčiny zablokování/nezavření kontextového menu:
1. Zavření menu na klik mimo v app.rs testuje pouze primary_clicked() (levý klik), secondary_clicked() (pravý klik) menu nezavře.
2. Geometrie kliku mimo počítá Rect::from_center_size(menu.screen_pos, [150.0, 150.0]), ačkoliv screen_pos je horní levý roh, takže spodní polovina menu byla detekována jako klik mimo a prostor nad menu jako uvnitř.
3. Plátno grafu (canvas.rs) při pravém kliku na prázdné plátno nevysílá žádnou akci pro zavření menu.
4. Chyběla podpora zavření na klávesu Escape.

Cíl: spolehlivé zavírání kontextového menu na klik mimo (levý i pravý), Escape i pravý klik do prázdna, a bezpečné přepínání uzlů bez uváznutí UI.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Kliknutí levým nebo pravým tlačítkem myši mimo kontextové menu toto menu zavře
- [x] #2 Pravý klik na prázdné plátno grafu otevřené kontextové menu zavře
- [x] #3 Pravý klik na jiný uzel/hranu otevře kontextové menu pro daný uzel na nové pozici
- [x] #4 Stisk klávesy Escape zavře otevřené kontextové menu
- [x] #5 Detekce kliknutí uvnitř menu používá skutečný rozměr zobrazeného okna menu, nikoli odhad ze screen_pos
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. canvas.rs: při secondary_clicked() bez hovered uzlu/hrany vyvolat akci pro zrušení menu (CanvasAction::SelectionCleared nebo ContextMenuDismissed).
2. app.rs: zaznamenat skutečný obdélník zobrazeného menu (z Frame::show); zavírat menu při libovolném kliku mimo tento rect (primary i secondary) nebo při stisku Escape.
3. tests/model_tests.rs: headless testy pokrývající pravý klik mimo, pravý klik na prázdné plátno, Escape a pravý klik na jiný uzel.
4. Ověřit clippy, fmt a testy.
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Zavírání kontextového menu plátna: Area::show response bounds nahrazuje chybný obdélník odhadovaný ze screen_pos; libovolný klik mimo (levý i pravý) a klávesa Escape menu zavírají s ochranou otevření ve stejném snímku. Pravý klik na prázdné plátno emituje SelectionCleared a menu zavře; pravý klik na jiný uzel/hranu přepne menu. Ověřeno testem test_context_menu_lifecycle_and_outside_clicks (všech 105 testů prochází, clippy a fmt bez varování), nezávislé review agent_review schváleno.
<!-- SECTION:FINAL_SUMMARY:END -->

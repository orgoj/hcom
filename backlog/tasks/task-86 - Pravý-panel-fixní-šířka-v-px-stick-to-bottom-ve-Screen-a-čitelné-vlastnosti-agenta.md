---
id: TASK-86
title: >-
  Pravý panel: fixní šířka v px, stick-to-bottom ve Screen a čitelné vlastnosti
  agenta
status: To Do
assignee: []
created_date: '2026-11-03 18:55'
labels: []
dependencies:
  - TASK-77
priority: medium
type: bug
ordinal: 123000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Uživatel: Pravý panel detailu mění šířku při změnách okna a někdy zmizel tak, že nešel otevřít. Šířka musí být stálá (v px). Záložka Screen má být zarovnaná odspodu (poslední řádky), vlastnosti agenta (Report/Summary) nesmí tlačit labely doprava a musí mít viditelné posuvníky (slidery).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Šířka pravého panelu je uložena a držena v pixelech (ne jako proměnný zlomek šířky okna); při změně velikosti okna zůstává šířka panelu stálá a panel nikdy nezkolabuje na nulu ani nezmizí
- [ ] #2 V záložce Screen je výstup zarovnán odspodu (stick_to_bottom), takže uživatel po přepnutí i aktualizaci vidí poslední řádky
- [ ] #3 V záložkách s vlastnostmi agenta (Report a Summary) jsou labely zarovnané vlevo, hodnoty nepřetlačují layout a ScrollArea má funkční posuvníky pro zobrazení všech údajů
<!-- AC:END -->

---
id: TASK-77.111
title: 'Dashboard: Start dialog ovladatelný z klávesnice'
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-11-13T21:32:00Z'
updated_date: '2026-11-13T21:44:00Z'
labels: []
dependencies: []
parent_task_id: TASK-77
type: enhancement
ordinal: 222000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Start dialog: Enter se konzumuje na začátku dialogu a odešle formulář i při otevřeném popupu; cli je egui ComboBox, model/reasoning jsou TextEdit + menu_button ▾ (vlastní), z klávesnice prakticky neovladatelné. Rozhodnutí uživatele: model/reasoning jako pole s našeptávačem (seznam odfiltrovaných návrhů pod polem, šipky vybírají, Enter/Tab vloží hodnotu do pole a fokus zůstane v poli k editaci, Esc seznam zavře), cli stejně ale jen povolené hodnoty; Enter odešle formulář jen bez otevřeného seznamu/popupu.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Esc při otevřeném seznamu návrhů zavře jen seznam, ne dialog
- [x] #2 cli: totéž ovládání, výsledná hodnota je vždy jedna z povolených CLI
- [x] #3 Enter bez otevřeného seznamu návrhů odešle formulář jako dosud
- [x] #4 model a reasoning: psaní, klik do pole nebo Down zobrazí návrhy filtrované podle textu; Up/Down vybírají, Enter vloží vybraný návrh do pole a neodešle formulář
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
cli/model/reasoning ve Start dialogu jsou suggest_field: textové pole + seznam návrhů pod ním (psaní/klik/Down otevře, Up/Down vybírá, Enter/klik vloží a nechá fokus, Tab vloží a pokračuje, Esc zavře jen seznam); dialog bere Enter jen bez otevřeného seznamu; cli přijme jen hodnotu ze seznamu. Review agent_review: přesně napsaná volba + Enter vybírala první položku – opraveno (zvýrazní se přesná shoda), regrese v test_task_77_111_start_dialog_suggestion_fields. README aktualizováno.
<!-- SECTION:FINAL_SUMMARY:END -->

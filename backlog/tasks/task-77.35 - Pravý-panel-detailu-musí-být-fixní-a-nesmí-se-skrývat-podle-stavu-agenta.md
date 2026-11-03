---
id: TASK-77.35
title: Pravý panel detailu musí být fixní a nesmí se skrývat podle stavu agenta
status: Done
assignee:
  - '@hcom_dev__idea'
created_date: '2026-11-03 13:05'
updated_date: '2026-11-03 13:25'
labels: []
dependencies: []
parent_task_id: TASK-77
type: bug
ordinal: 119000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Uživatel zjistil, že posun plátna při označení agenta je způsoben tím, že se pravý panel skrývá, když agent není aktivní. Pravý panel se smí skrývat jedině explicitním přepínačem v toolbaru. Pokud agent není aktivní nebo není vybrán, v panelu má zůstat stálý obsah (legenda, info, atd.) a panel nesmí měnit svou viditelnost ani způsobovat skákání plátna.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Pravý detail panel se zobrazuje vždy, pokud není explicitně vypnut tlačítkem v toolbaru (show_side_panel)
- [x] #2 Při výběru neaktivního agenta nebo zrušení výběru zůstává pravý panel otevřený a zobrazuje příslušný obsah (např. Legendu / info)
- [x] #3 Přepínání výběru agentů nezpůsobuje otevírání/zavírání pravého panelu ani posun plátna
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Right detail panel is now fixed and persistent: removed in-panel close button, toggling is strictly via toolbar Detail switch, selection generation invalidated on selection change, stopped agent states explicitly reported without infinite loading, regression test added.
<!-- SECTION:FINAL_SUMMARY:END -->

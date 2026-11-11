---
id: TASK-77.104
title: Rychlý výběr agenta z filtru tabulky (Enter, Shift+Enter, šipky)
status: In Progress
assignee:
  - '@hcom_dev'
created_date: '2026-11-11T13:07:00Z'
updated_date: '2026-11-11T13:22:00Z'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: medium
type: feature
ordinal: 204000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Požadavek uživatele: rychle vybrat a spustit agenta z tabulky bez přejíždění z filtru do tabulky. Postup: / , kousek jména, Enter. Dnes: / fokusuje search (app.rs ~4449), Alt+Up/Down posouvají výběr i z pole, Alt+X maže filtr, Enter v tabulce attachuje běžícího / otevře Start pro zastaveného, ale ne z pole; s = Start… (ale jen mimo text field). Alt+písmena zamítnuto (držení Alt, kolize s Alt+X, diakritika). Shift+Enter = Start dialog (rozhodnutí uživatele), konzistentně v tabulce i filtru.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Enter ve vyhledávacím poli tabulky udělá s vybraným řádkem totéž co Enter v tabulce (attach běžícího, Start dialog zastaveného katalogového agenta)
- [x] #2 Shift+Enter v tabulce i ve vyhledávacím poli tabulky otevře Start dialog pro katalogového agenta vybraného řádku i když běží (jako s / Start…); bez katalogového agenta nic
- [x] #3 Up/Down bez Alt ve vyhledávacím poli tabulky posouvají výběr tabulky
- [x] #4 Po změně textu vyhledávání se vybere první zobrazený řádek, pokud vybraný řádek filtr skryl
- [x] #5 Escape opustí pole a filtr ponechá; Alt+X filtr maže beze změny
- [x] #6 README dashboardu popisuje nové klávesy; headless testy pokrývají každé kritérium
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implementováno v app.rs (render_filtered_table: SearchKeys z pole filtru, select_first_shown; render_table: Shift+Enter = Start). Testy: test_task_77_104_table_search_quick_select, upraveny test_task_77_50 (Enter ve filtru attachuje), test_task_77_53 a test_table_arrow_keys_select_rows_in_shown_order (šipky ve filtru posouvají výběr). Gates fmt/clippy/test OK (test_overload_10k flaky pod zátěží, izolovaně OK). Review agent_review OK bez nálezů. Čeká na kontrolu uživatelem, pak Done.
<!-- SECTION:NOTES:END -->

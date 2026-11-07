---
id: TASK-77.56
title: 'Dashboard: dialog Start stejný jako odladěný dialog v hcom-gui'
status: Done
assignee:
  - '@hcom_dev__idea'
created_date: '2026-11-07T08:42:00Z'
updated_date: '2026-11-07T09:20:00Z'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: high
type: bug
ordinal: 151000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Dialog Start v dashboardu (tlačítko Start) neodpovídá odladěnému dialogu Start v contrib/hcom-gui. Rozdíly hlášené uživatelem: neotevírá se uprostřed obrazovky; chybí rozbalovací (popup) volby jako v hcom-gui; jméno agenta se předvyplní jen po kliknutí (fokusu) do pole jména. Vzorem je chování dialogu v hcom-gui. Souvisí s TASK-77.55 (otevření dialogu dvojklikem/Enterem).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Dialog Start se otevře uprostřed obrazovky
- [x] #2 Dialog nabízí stejné rozbalovací volby jako dialog Start v hcom-gui
- [x] #3 Ostatní pole a jejich předvyplnění se chovají jako v dialogu Start v hcom-gui
- [x] #4 Pole jména instance je po otevření prázdné a po fokusu dostane '<agent>__' jako v hcom-gui; ponechané beze změny se nepředá
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implementováno podle hcom-gui ask(): uprostřed okna, resume/name/dir/cli (combo z PATH)/model+reasoning (▾ návrhy)/prompt, Enter OK, Shift+Enter nový řádek, Esc zrušit, nezměněné odpovědi bez flagu, --continue při změně CLI. Start jen pro katalogového agenta jako v GUI. Testy test_task_77_55_56_*, test_task_77_56_*; review agent_review OK. Otevřené: AC #3 – chybí Browse… u dir (dashboard nemá souborový dialog; rfd nebo zenity); čeká na rozhodnutí uživatele.

Browse… u dir: systémový výběr adresáře přes rfd 0.17.2 (xdg-desktop-portal), ve vlákně, výsledek přes mpsc; review agent_review OK. Nevynucené poznámky review (rx po zavření dialogu žije do dalšího otevření; ~ v dir se neexpanduje pro výchozí adresář chooseru) nechány: kosmetické.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Dialog Start podle hcom-gui: uprostřed okna, resume/name (<agent>__ po fokusu)/dir s Browse… (systémový portal)/cli combo z PATH/model+reasoning s nabídkou ▾/prompt; Enter OK, Esc zrušit, nezměněné odpovědi bez flagu, --continue při změně CLI. Testy test_task_77_55_56_*, test_task_77_56_*; review OK.
<!-- SECTION:FINAL_SUMMARY:END -->

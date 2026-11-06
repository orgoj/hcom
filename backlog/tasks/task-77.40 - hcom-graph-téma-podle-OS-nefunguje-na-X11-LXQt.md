---
id: TASK-77.40
title: 'hcom-graph: téma podle OS nefunguje na X11/LXQt'
status: Done
assignee:
  - '@hcom_dev__idea'
created_date: '2026-11-06T17:53:00Z'
updated_date: '2026-11-06T17:57:00Z'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: high
type: bug
ordinal: 133000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Na LXQt/X11 winit nevrací systémové téma a desktop portal hlásí color-scheme 0 (bez preference), takže Auto vždy spadne na tmavé, ačkoli má uživatel světlé téma. Ostatní aplikace ho poznají: Qt aplikace (QT_QPA_PLATFORMTHEME=lxqt) z [Palette] v ~/.config/lxqt/lxqt.conf, GTK aplikace z ~/.config/gtk-3.0/settings.ini (gtk-theme-name, gtk-application-prefer-dark-theme). TASK-77.37 #2 byl odškrtnutý jen na základě headless testu.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Když eframe systémové téma nehlásí, Auto ho určí z jasu window_color v [Palette] souboru lxqt/lxqt.conf (pod XDG_CONFIG_HOME, jinak ~/.config)
- [x] #2 Když LXQt paleta chybí, Auto ho určí z gtk-3.0/settings.ini: gtk-application-prefer-dark-theme=true nebo název gtk-theme-name obsahující dark znamená tmavé, jiný uvedený název světlé
- [x] #3 Bez jakéhokoli zdroje zůstává tmavé téma jako dosud
- [x] #4 Testy pokrývají světlou a tmavou LXQt paletu, GTK světlý a tmavý název, prefer-dark a chybějící soubory
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Auto falls back to the desktop config when eframe reports no theme: LXQt [Palette] window_color luminance, then GTK 3 settings.ini (prefer-dark, 'dark' in gtk-theme-name), read once at start; none -> dark. Tests test_task_77_40_* cover light/dark palette, GTK names, prefer-dark, missing files, source order and the apply_theme fallback. Review agent_review OK.
<!-- SECTION:FINAL_SUMMARY:END -->

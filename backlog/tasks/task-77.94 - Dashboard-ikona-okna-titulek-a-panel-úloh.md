---
id: TASK-77.94
title: 'Dashboard: ikona okna (titulek a panel úloh)'
status: Done
assignee:
  - '@hcom_dev__idea'
created_date: '2026-11-07T19:56:00Z'
updated_date: '2026-11-07T19:58:00Z'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: medium
type: feature
ordinal: 190000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Uživatel 2026-11-07: hcom-dashboard nemá ikonu v titulku okna ani v panelu úloh LXQt (X11). eframe ViewportBuilder::with_icon(IconData) nastaví _NET_WM_ICON, které čte titulek okna i panel. Ikona se kreslí v kódu (bez souboru s obrázkem): motiv grafu, tři uzly s barevnými prstenci CLI (claude, codex, gemini) spojené hranami na tmavém zaobleném čtverci. Mimo rozsah: .desktop soubor, ikona v menu.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Okno hcom-dashboard má ikonu (ViewportBuilder::with_icon), kterou X11 správce oken a panel úloh zobrazí
- [x] #2 Ikona je 64×64 RGBA kreslená v kódu: tmavý zaoblený čtverec s průhlednými rohy, tři uzly v barvách CLI spojené hranami
- [x] #3 Test ověřuje rozměry, průhledné rohy a barvy uzlů; README zmiňuje ikonu
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
New src/icon.rs draws a 64x64 RGBA icon in code (rounded square with transparent corners, 3 edges, 3 CLI-colored node rings); main.rs sets it via ViewportBuilder::with_icon (_NET_WM_ICON for title bar and taskbar). Test test_task_77_94_window_icon, README. Reviewed OK by agent_review.
<!-- SECTION:FINAL_SUMMARY:END -->

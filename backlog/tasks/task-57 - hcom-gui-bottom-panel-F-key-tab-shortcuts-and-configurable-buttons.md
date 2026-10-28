---
id: TASK-57
title: 'hcom-gui bottom panel: F-key tab shortcuts and configurable buttons'
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-10-28 06:16'
updated_date: '2026-10-28 06:35'
labels: []
dependencies: []
priority: medium
type: enhancement
ordinal: 58000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
hcom-gui bottom panel: the tabs Screen, Transcript, Events, Log get F-key shortcuts, and the key bar under the Screen tab becomes configurable in ~/.hcom/gui.json. User decisions: tabs take F5-F8, so Refresh moves from F5 to Ctrl+R; gui.json can pick and order the built-in bar buttons and also add custom key buttons. Out of scope: configurable tab keys, custom bar buttons running anything other than hcom term inject, changes to the top toolbar.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 F5, F6, F7, F8 open Screen, Transcript, Events, Log; each tab label shows its key, e.g. 'Screen (F5)'; saved bottom_tab and Log error dot keep working
- [x] #2 Refresh runs on Ctrl+R (also with Caps Lock), no longer on F5; its button reads 'Refresh (Ctrl+R)'
- [x] #3 gui.json 'screen_bar' lists the bar buttons in order: a string names a built-in (Inject, Esc, ↑, ↓, ←, →, Tab, Shift+Tab, Enter, 1, 2, 3); an object {"label": ..., "keys": ...} adds a button that runs hcom term inject <instance> <keys> for a running instance; without screen_bar the bar is unchanged
- [x] #4 An unknown built-in name, an invalid object, a duplicate label or a non-list screen_bar is reported as ERROR in the Log and skipped (non-list: default bar)
- [x] #5 contrib/hcom-gui docstring, contrib/README.md and contrib/test-hcom-gui cover the new keys and screen_bar
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. current_tab() returns the tab key by index instead of the label text; tab labels get the key; bind F5-F8 on the window. 2. Refresh: bind <Control-r>/<Control-R>, label from key_label('<Control-R>'). 3. screen_bar parsing in App.__init__ next to toolbar_breaks: build the ordered bar list from BUTTONS bar items plus custom objects; self.buttons = non-bar buttons + chosen bar buttons. 4. Tests in contrib/test-hcom-gui, docs in docstring and README.
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Tabs Screen/Transcript/Events/Log open with F5-F8 and show the key in their label; current_tab() maps by index so bottom_tab state and the Log error dot are unchanged. Refresh moved to Ctrl+R (also Caps Lock). gui.json screen_bar picks/orders built-in bar buttons and adds {label, keys} inject buttons; invalid items, duplicates and non-list values are logged. Verified with contrib/test-hcom-gui (152 OK); reviewed by agent_review.
<!-- SECTION:FINAL_SUMMARY:END -->

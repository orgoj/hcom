---
id: TASK-30
title: >-
  Jak resit aby na linux gui agent atach prepnul i na terminal ve kterem herdr
  bezi?
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-10-16 11:34'
updated_date: '2026-10-16 12:01'
labels: []
dependencies: []
type: feature
ordinal: 29000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
hcom agent attach focuses only the Herdr pane; the terminal window Herdr runs in stays behind. hcom stays unchanged: hcom-gui gets a per-terminal Attach command in gui.json, so raising the window works with any desktop, platform or terminal backend.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Attach commands can use {instance}, {agent}, {dir}, {terminal} and text fields of launch_context; a missing placeholder or an entry that is not a list of strings logs an error and runs nothing
- [x] #2 contrib/README.md and the script docstring document attach with a working X11 example
- [x] #3 gui.json "attach" maps a terminal (launch_context terminal_preset_effective) to the Attach command
- [x] #4 Terminals without an entry (Hermes gateway, tmux, ...) run hcom agent attach {instance} as before; Attach stays enabled for every running instance
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Machine side (agent bin, ~/bin/xonstart): Herdr starts after login via 'wezterm start --always-new-process --class herdr -- herdr'; bin verified 'wmctrl -x -a herdr.herdr' raises it. Enable here with gui.json attach.herdr = [sh, -c, 'hcom agent attach "$1" && wmctrl -x -a herdr.herdr', _, {instance}].
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
hcom-gui: Attach command per terminal from gui.json "attach"; other terminals run hcom agent attach {instance} as before; launch_context placeholders, errors logged. Verified with contrib/test-hcom-gui (65 OK, 3 new tests).
<!-- SECTION:FINAL_SUMMARY:END -->

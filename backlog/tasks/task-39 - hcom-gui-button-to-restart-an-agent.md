---
id: TASK-39
title: 'hcom-gui: button to restart an agent'
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-10-20 11:41'
updated_date: '2026-10-20 11:49'
labels: []
dependencies: []
type: feature
ordinal: 38000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Goal: restart the selected agent from hcom-gui in one step (e.g. after changing its catalog entry or when it misbehaves), instead of Kill followed by Start.

The binary already does this: `hcom agent <agent> --as <instance> --restart` kills the running instance, waits up to 15 s for its process to exit, and launches the catalog agent again under the same instance name with the catalog's session settings (resume/continue/clean). The GUI only adds a button that runs it; nothing in src/ changes.

In scope:
- A Restart button in BUTTONS, after Kill, enabled for a selected row with both a catalog agent and a running instance (including --as and roaming instances).
- It runs `hcom agent <agent> --as <instance> --restart` with no session flag, after the same Yes/No confirmation as Kill ("Restart <instance>?").
- Roaming agents resolve their project from the cwd, so for them the command runs in the instance directory; other agents keep the catalog cwd used by Start.
- README (contrib/README.md), the module docstring if it lists buttons, and contrib/test-hcom-gui cases.

Out of scope: instances without a catalog agent (plain hcom claude etc.; button disabled), stopped catalog rows (Start already covers them, including resume), choosing resume/continue/clean or another CLI in a dialog, a keyboard shortcut, restarting several instances or a group, any change to the hcom binary.

Risks: the command blocks while it waits for the old process (up to 15 s) plus the launch; it runs through run_command (timeout 120 s) like Start, with the busy cursor. hcom agent resolves the catalog entry before killing, so an agent no longer in the catalog only logs the error and keeps running.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 With a running catalog instance selected, Restart is enabled; with a stopped catalog row, a running instance without a catalog agent, or no selection, it is disabled.
- [x] #2 Restart asks a Yes/No confirmation "Restart <instance>?"; No runs nothing.
- [x] #3 Yes runs exactly `hcom agent <agent> --as <instance> --restart` (no --resume/--continue/--clean), in the catalog cwd for a non-roaming agent and in the instance directory for a roaming agent; output goes to the log.
- [x] #4 contrib/README.md documents the Restart button, and contrib/test-hcom-gui covers every case above.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. contrib/hcom-gui: add a "roaming" field to rows from the catalog; add a when value for running catalog instances (agent and instance); add the Restart button with confirm; use the instance dir as cwd for roaming rows.
2. contrib/test-hcom-gui: cases for enabled/disabled rows, confirm No, recorded command and cwd for a normal and a roaming agent.
3. contrib/README.md (and docstring) update.
4. Run contrib/test-hcom-gui; inspect the diff.
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added a Restart button to hcom-gui (when: live = running instance of a catalog agent) running `hcom agent <agent> --as <instance> --restart` after a Yes/No confirmation; instance rows of roaming agents now use the instance directory as cwd. README documents it. Verified with contrib/test-hcom-gui (72 OK, 4 new cases: enabled/disabled rows, confirm and command, decline, roaming cwd) and `hcom agent hcom_dev --as hcom_dev --restart --dry-run` printing kill + relaunch.
<!-- SECTION:FINAL_SUMMARY:END -->

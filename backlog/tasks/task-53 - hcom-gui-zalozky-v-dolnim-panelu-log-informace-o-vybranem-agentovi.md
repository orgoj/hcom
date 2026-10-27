---
id: TASK-53
title: 'hcom-gui: zalozky v dolnim panelu (log + informace o vybranem agentovi)'
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-10-27 19:22'
updated_date: '2026-10-27 19:34'
labels: []
dependencies: []
type: feature
ordinal: 54000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Goal: see what the selected agent is doing without Attach. The bottom log panel of contrib/hcom-gui becomes a ttk.Notebook with tabs Screen, Transcript, Events, Log (in this order); Log keeps today's content.

Sources (existing hcom CLI, the GUI holds no hcom logic): Screen = hcom term <instance> (PTY instances only; otherwise hcom's own message is shown); Transcript = hcom transcript <instance> (last 10 exchanges, hcom default); Events = hcom events --agent <instance> --last 50, one line per event (time, type, short detail).
How much is visible follows the panel size: content is loaded once and shown scrolled to its end.

In scope: the notebook, the three agent tabs, the Log error marker, remembering the selected tab in gui-state.json, README/docstring, tests.
Out of scope: rows without a running instance (agent tabs stay empty), message-only or config tabs, tool I/O (--detailed/--full), remote-specific handling beyond what hcom returns.
Risks: hcom term/transcript can be slow or fail; calls run off the Tk thread, one at a time, and failures show in the tab, not as dialogs.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The bottom panel has tabs Screen, Transcript, Events, Log in this order; the selected tab is kept in ~/.hcom/gui-state.json (first start: Log)
- [x] #2 Screen shows hcom term output, Transcript hcom transcript output, Events hcom events --agent output formatted one line per event, for the selected row's instance
- [x] #3 A row without a running instance, or no selection, leaves the three agent tabs empty
- [x] #4 An open agent tab reloads on row selection change and on every refresh; hidden agent tabs run no hcom command; a reload never overlaps a running one
- [x] #5 After a reload the view is at the end, unless the user had scrolled away from the end, then the scroll position stays
- [x] #6 A command failure shows its error text in that tab
- [x] #7 A new ERROR line in the log while another tab is open marks the Log tab red without switching tabs; opening the Log tab clears the mark
- [x] #8 contrib/test-hcom-gui covers each AC; contrib/README.md documents the tabs
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Replace the log Text pane with ttk.Notebook: Screen/Transcript/Events Text widgets (read-only, monospace, scrollbar) and the existing Log.
2. load_tab(): selected agent tab + selected instance -> background thread runs the hcom command (HCOM_ENV, cwd HOME, timeout), _tab_done() fills the widget if selection/tab still match; pending flag prevents overlap (rerun once if requested meanwhile).
3. Hooks: <<NotebookTabChanged>>, _on_select, _apply (refresh).
4. Events formatting: parse JSON lines, 'HH:MM:SS type detail'.
5. Scroll: keep at end only if yview()[1] == 1.0 before the update.
6. Log mark: write_log on ERROR while Log not selected -> red dot image on the Log tab; cleared on tab change to Log.
7. gui-state.json: bottom_tab. Tests, README, docstring.
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Bottom panel of contrib/hcom-gui is a ttk.Notebook: Screen (hcom term), Transcript (hcom transcript), Events (hcom events --agent --last 50, one HH:MM:SS type detail line per event), Log. Only the open agent tab loads, off the Tk thread, one load at a time (a request during a load reruns once), stale results dropped; reload on selection change and every refresh; scroll kept unless at the end; failures (stderr, else stdout) shown in the tab. New ERROR log line while another tab is open puts a red dot on Log until opened. Selected tab kept in gui-state.json bottom_tab (default Log). Tests: 10 new cases, 133 OK. Docs: README hcom-gui section, module docstring. Reviewed by agent_review.
<!-- SECTION:FINAL_SUMMARY:END -->

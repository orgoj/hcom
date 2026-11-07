---
id: TASK-77.90
title: 'Dashboard Sessions: zobrazit CLI a parametry spuštění session'
status: Done
assignee:
  - '@hcom_dev__idea'
created_date: '2026-11-07T17:54:00Z'
updated_date: '2026-11-07T19:04:00Z'
labels: []
dependencies: []
parent_task_id: TASK-77
priority: medium
type: feature
ordinal: 185000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Uživatel 2026-11-07: v záložce Sessions není vidět, jakým agent CLI byla session spuštěna, takže uživatel nepozná, kterou session chce. Přání: u každé session zobrazit parametry, se kterými byla spuštěna (CLI, argumenty). Suggested direction: zjistit, co o session ukládá hcom (life events / instance řádek: tool, launch args) a co Sessions tab dnes načítá (ActionRequest::Sessions v action.rs, sessions.rs). Needs analysis before implementation.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 hcom list --json (full listing) has a launch_args field: the instance's launch arguments as a JSON array of strings, [] when none recorded; --format accepts {launch_args}; hcom list --help lists the field
- [x] #2 A Sessions card shows a line with the CLI, the model and the reasoning effort taken from the session's launch arguments (--model/-m, --effort, -c model_reasoning_effort=...), leaving out those not present; the CLI no longer repeats on line 2
- [x] #3 Hovering that line shows the complete launch arguments; without recorded arguments the line shows only the CLI and no tooltip
- [x] #4 Ended sessions take the arguments from the stopped snapshot, the running session from hcom list --json
- [x] #5 Headless tests cover the arg parsing (claude, codex, missing args) and the card line; README, built-in help and skills docs updated
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Binary: add launch_args (parsed array) to the full-listing payload in src/commands/list.rs, help field list in src/commands/help.rs, test, docs, version bump.
2. Dashboard: SessionInfo/LiveSession launch_args: Vec<String>; parse from snapshot and list json; helper launch_summary(tool,args); card line with tooltip; drop tool from line 2.
3. Tests, README, gates, review, commit, release builds of both.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Analysis 2026-11-07: card line 2 already shows the tool (snapshot.tool / list tool). Each stopped life event snapshot carries launch_args (JSON string array, e.g. --model gpt-5.6-terra -c model_reasoning_effort="medium") and agent_name; the running session's launch_args exist only in the instance row, hcom list --json does not expose them. User decisions: show CLI + model/effort extracted from the args with the full args in a tooltip; add launch_args to hcom list --json/--format (binary change) so the running session shows them too. Out of scope: catalog agent name, prompt, env vars, editing args, hcom list <name> single-agent payload.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
hcom list --json/--format expose launch_args (parsed array, [] when none; help updated; smoke test list_json_and_format_expose_launch_args). hcom-dashboard Sessions card line 2 shows CLI · model · effort from the launch args (stopped snapshot, running session via hcom list), full args on hover, CLI dropped from the meta line; tests test_task_77_90_*, 77.81 expectations updated, README. Gates green; reviewed OK by agent_review.
<!-- SECTION:FINAL_SUMMARY:END -->

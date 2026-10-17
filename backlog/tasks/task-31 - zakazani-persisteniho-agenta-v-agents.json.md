---
id: TASK-31
title: zakazani persisteniho agenta v agents.json
status: Done
assignee: []
created_date: '2026-10-16 12:17'
updated_date: '2026-10-17 09:50'
labels: []
dependencies: []
type: feature
ordinal: 30000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Allow switching a catalog agent off without deleting its definition: "disabled": true makes hcom treat it as if it were not defined. The field merges like other agent fields (defaults, global, project, imports; a later layer wins, so false re-enables). Only the catalog side is affected: an already running instance stays an ordinary hcom instance (messageable, killable by name); it is just never started from the catalog again. Per the fork rule, upstream behavior is untouched.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 An agent with "disabled": true is not started by hcom agent <name>, send autostart, group launch, hcom r/resume via catalog, nor materialized as roaming; each reports it as an unknown agent, exactly like an undefined name
- [x] #2 A disabled agent is absent from hcom agent list (incl. --names, --groups, @group, --local), group membership for send/kill/launch, shell completions, hcom agent show, did-you-mean suggestions and available-agent lists
- [x] #3 hcom agent list --all shows a disabled agent marked as disabled
- [x] #4 "disabled" merges across defaults, global, project and imported catalogs with the usual precedence; a later layer setting false re-enables the agent
- [x] #5 A running instance of a disabled agent keeps receiving messages and can be killed by name; disabling never stops or reroutes it
- [x] #6 agent help, README, docs and skills/hcom-agent-messaging document the field; cli_smoke covers launch, send autostart, group, list --all and merge override
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Catalog field "disabled" (Option<bool>, merged by take!). Catalogs::resolve/names/local_names filter disabled; resolve_any/names_any/local_names_any used only by agent list --all (table "disabled", brief "(disabled)", JSON "disabled": bool). --names/--groups/@group stay enabled-only. Running instances untouched (upstream send/kill/attach). Docs: agent help, top-level overview, README, skill. Tests: unit disabled_merges_by_layer_and_hides_the_agent, help agent_help_documents_disabled_entries, cli_smoke agent_disabled_entry_is_treated_as_undefined and running_instance_of_a_disabled_agent_is_still_messageable_and_killable.
<!-- SECTION:FINAL_SUMMARY:END -->

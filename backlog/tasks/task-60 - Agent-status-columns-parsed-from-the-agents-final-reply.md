---
id: TASK-60
title: Agent status columns parsed from the agent's final reply
status: To Do
assignee:
  - '@hcom_dev'
created_date: '2026-10-29 05:22'
updated_date: '2026-10-29 05:45'
labels: []
dependencies: []
priority: medium
type: feature
ordinal: 61000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Goal: show each agent's self-reported state (e.g. the STATUS/Done/Issues/Next block agents are prompted to end every turn with) in hcom without any tool call, mainly so agent_manager sees who is in which state, including stopped agents and states that scrolled off the screen.

Design (agreed with the user):
- Global column list in config.toml, edited by hand: [[status_column]] tables with name, regex and reset = "keep" (default) | "turn". Not managed by 'hcom config <key> <value>'.
- Extraction when the agent goes idle (stop / listening), from the text of its last assistant reply: Claude and Codex from the Stop hook payload field last_assistant_message; other CLIs from the last exchange of their transcript (transcript::read with last=1). The regex runs multi-line; the value is the first capture group (whole match without a group) of the last match, trimmed.
- keep: the stored value stays until a later reply matches again (a turn started by another agent's message without the block does not erase it). turn: the value is cleared when the agent starts a new turn (status active).
- Storage in the kv table per instance name (key prefix status_columns:), value plus updated_at, so values survive stop and need no schema change.
- 'hcom agent list --status' joins each catalog agent with its most recently updated instance (launch_context agent_name, covers roaming instances); columns are shortened in the table and brief view, full values and updated_at only in --json.
- Fork fallback: upstream code gets only minimal hooks (one call where idle/active status is set); everything else lives in a new fork module.

Out of scope: remote/relay agents (documented as not implemented), hcom-gui columns (separate follow-up), per-agent or per-group column definitions, any tool call or agent-side command, 'hcom list' changes.

Risks: confirm on a real Claude Stop payload that last_assistant_message is present (otherwise Claude uses the transcript path); transcript parse cost at idle for large transcripts must be measured; config.toml validation must accept the new array of tables.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 With [[status_column]] name="status" regex="^STATUS:\s*(\S+)" configured, after an agent's turn ending with 'STATUS: WAITING' 'hcom agent list --status' shows WAITING for that agent
- [ ] #2 Claude and Codex values come from the Stop payload last_assistant_message; another transcript-backed CLI gets its value from the last transcript exchange; tests cover both paths
- [ ] #3 reset=keep: a later reply without a match leaves the previous value; reset=turn: the value is empty once the agent starts a new turn
- [ ] #4 A stopped agent still shows its last values in 'hcom agent list --status'
- [ ] #5 The table and brief (non-TTY) view shorten long values; --json carries full values and updated_at per column
- [ ] #6 No [[status_column]] configured: hooks do no extraction work and 'hcom agent list' output is unchanged; an invalid regex is reported by hcom (status/config check) and does not break hooks
- [ ] #7 Built-in help, README, docs and skills/hcom-agent-messaging describe the config and --status, including that remote/relay agents are not implemented
- [ ] #8 --json lists, for each catalog agent, the values of every instance that has any (live and stopped, roaming included) keyed by instance name, with the most recent instance marked, so hcom-gui can fill per-instance rows
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Verify a real Claude and Codex Stop payload carries last_assistant_message (debug log or test against mock tools). 2. New module status_columns: config load/validate, extract(text) -> values, kv store/clear. 3. Hook calls: Claude poll/Stop and Codex Stop pass payload text; other CLIs read the last transcript exchange; on active status clear reset=turn columns. 4. hcom agent list --status (table, brief, --json) via launch_context agent_name join. 5. Measure transcript parse cost at idle on a large real transcript. 6. Tests (unit for extraction/reset, cli_smoke for agent list --status), help/docs/skill. 7. Review with agent_review before implementation (non-small change).
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
User decisions (2026-10-29): extract at idle and store; staleness handled per column definition: option whether the value persists until a later reply matches again (default, because a turn triggered by another agent's message may not repeat the block) or is cleared when the agent starts a new turn; 'hcom agent list --status' joining the catalog with each agent's last instance, plus --json; no tool call (passive regexp only); fork fallback with minimal upstream touch. Table shows values shortened, full values only in --json. One global column list in hcom config. Remote/relay agents out of scope (document as not implemented).

Spec addition for TASK-61: hcom-gui rows are per instance (roaming has several), so --json carries per-instance values; instances without a catalog agent (adhoc) are not covered.
<!-- SECTION:NOTES:END -->

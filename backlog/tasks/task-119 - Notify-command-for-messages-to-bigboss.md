---
id: TASK-119
title: Notify command for messages to bigboss
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-11-18T17:33:00Z'
updated_date: '2026-11-18T18:12:00Z'
labels: []
dependencies: []
priority: high
type: feature
ordinal: 230000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
User need (2026-11-18): agent results sent to bigboss (e.g. the cron-driven knowledge_vulncheck threat check, sent with send -b) go unnoticed because the user does not watch hcom. Today nothing notifies: hcom only stores the message; events sub delivers only as an hcom message. Decision (user, variant 2): an optional hcom config key, e.g. bigboss_notify_command = ["notify-send", "hcom: {from}", "{text}"] (argv list, placeholders as single tokens, no shell). When a message event has bigboss among its recipients, the sending hcom process spawns the command detached (no daemon; a sender inside Herdr has the desktop session, so notify-send works; mail or a custom script also fit). Unset = no change. notify-send, mail and sendmail exist on this machine. Opt-in fork feature: keep the hook in upstream send code minimal.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 With the key unset, sending to bigboss behaves exactly as before, covered by a test
- [x] #2 With the key set, a message whose recipients include bigboss runs the command once, detached, with {from}, {text} (and the message id) substituted as single argv tokens; messages not addressed to bigboss run nothing; covered by tests
- [x] #3 A failing or missing notify command never fails or delays the send; the failure is logged
- [x] #4 hcom config help, README, docs and skills/hcom-agent-messaging document the key with a notify-send example
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Config key bigboss_notify_command (Str, shell-quoted like *_args, not exported to agents' env, read at send time). 2. src/bigboss_notify.rs builds argv ({id},{from},{text} substituted per token, {text} last), spawns detached (null stdio, own process group, reaped by a thread), logs bigboss_notify.failed. 3. Hook in send_message_with_catalog_outcome after log_event when effective_mentions contains bigboss (broadcasts excluded). 4. Help/--info/README/skill, unit + cli_smoke tests. 5. Review agent_review, commit, release build, user sets live config.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Deviation from the spec example: the value is a shell-quoted string (same convention as claude_args), not a TOML array; config fields are string/int/bool only. 'Recipients include bigboss' = event mentions contain bigboss (delivered_to never lists bigboss); broadcasts do not notify.

Review agent_review_hcom_884 #392574: broadcast finding rejected (see review:rejected task); exit-status finding resolved by stating in --info that only start failures are logged.

Follow-up (user): {preview} placeholder, 200 chars one line; live config set to notify-send -u critical -a bigboss {from} {preview}. Review OK #392679. Deployed 0.7.30-orgoj.54.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Opt-in config key bigboss_notify_command: the sending hcom process spawns it detached for every message whose mentions include bigboss; {from} {text} {id} substituted as single argv tokens, no shell; start failures logged (bigboss_notify.failed); unset = no change. Docs: config --info/--help, README, skill. Tests: unit (argv) + cli_smoke bigboss_notify_command_runs_only_for_messages_to_bigboss.
<!-- SECTION:FINAL_SUMMARY:END -->

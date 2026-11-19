---
id: TASK-125
title: 'hcom-dashboard: panel for messages to bigboss'
status: In Progress
assignee: []
created_date: '2026-11-19T11:36:00Z'
updated_date: '2026-11-19T20:43:00Z'
labels: []
dependencies: []
priority: high
type: feature
ordinal: 236000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
User request 2026-11-19: agents write to bigboss (the user) and those messages get lost: the user reads them nowhere. Goal: an Inbox in contrib/hcom-dashboard that lists messages for bigboss so none is missed. Related: TASK-119 (bigboss_notify_command, Done).

Decisions (user, 2026-11-19):
- Placement: a new right detail-panel tab "Inbox" (DETAIL_TABS in config.rs, labels in render_detail_tabs in app.rs), global, independent of the selected agent. Its tab label shows the unread count, like the Log tab error mark.
- Sources, each a toggle in the tab, persisted in dashboard-state.json: (a) direct: event mentions contain bigboss (same rule as TASK-119); (b) replies: reply_to points to a message sent by bigboss; (c) broadcasts: messages without recipients. Default: (a) and (b) on, (c) off.
- Read state: per message, toggled by hand; persisted in dashboard-state.json, keyed by message id plus timestamp (ids restart after a database reset). A "Mark all read" button marks every message shown by the current filter. New and historical messages start unread.
- Actions: clicking a message selects its sender agent (table and graph); a Reply button opens the existing Send dialog prefilled with @sender and sends hcom send -b @sender --reply-to <id>.

Data: messages come from the follower stream (source.rs build_messages_follow_cmd replays the whole DB) as MessageEvent (model.rs: from, mentions, delivered_to, intent, reply_to as "42" or "7:BOXE", text). The Inbox needs the raw event fields; StoredMessage on edges has no mentions/reply_to, so keep a separate inbox list filled in apply_message_event.

Out of scope: changes to the hcom binary, desktop notifications (TASK-119 covers them), an Inbox for agents other than bigboss, deleting messages.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A right detail-panel tab Inbox lists, newest first, the messages matching the enabled sources, with time, sender, intent and text; it does not depend on the selected agent
- [x] #2 Source toggles direct (mentions contain bigboss), replies (reply_to a message from bigboss) and broadcasts (no recipients) filter the list; default direct and replies on, broadcasts off; the choice survives a restart (dashboard-state.json)
- [x] #3 Each message can be marked read or unread by hand; unread ones are visually distinct; the state survives a restart and is keyed by id plus timestamp, so a database reset does not mark new messages read
- [x] #4 Mark all read marks every message shown by the current filter as read
- [x] #5 The Inbox tab label shows the number of unread messages in the current filter; zero shows no mark
- [x] #6 Clicking a message selects its sender agent; a message from an agent not in the table changes nothing and logs no error
- [x] #7 Reply opens the Send dialog prefilled with the sender and sends hcom send -b @sender --reply-to <id> -- <text>
- [x] #8 Headless tests cover the source classification, filter defaults, read-state persistence and the reply command; dashboard fmt, clippy --all-targets and tests pass
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. model.rs: InboxMessage (id, ts, from, intent, text, reply_to, kind flags direct/reply/broadcast) collected in apply_message_event; reply detection via a map id->sender for messages from bigboss (reply_to parsed as leading integer; relay suffix ignored).
2. config.rs: DashboardState fields inbox_sources (3 bools, defaults) and inbox_read (set of "id@ts" keys); add Inbox to DETAIL_TABS.
3. app.rs: render_inbox_tab (toggles, Mark all read, list with read checkbox, click selects sender, Reply opens send dialog with reply_to); tab label with unread count.
4. Send dialog: optional reply_to field appended as --reply-to.
5. Tests (headless), gates, review agent_review, commit, release build (deploy), visual check by @app_tester.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-11-19: implemented (model.rs collect_inbox/InboxMessage, config.rs InboxSources+inbox_read, app.rs render_inbox_tab, send_command with --reply-to), 4 tests test_task_125_*, README updated. Deviation from plan step 1: replies match only a plain local reply_to id; a relayed '7:BOXE' names a message on another device, and reply_to_local carries the sender DB value for relayed events. Next: agent_review OK -> commit -> release build -> visual check by @app_tester.

agent_review OK (#403618); its non-blocking note applied: clicking the message text also selects the sender.

Visual check by app_tester OK (steps 1-6 on live data, no clipping at 260 px; Broadcasts toggle showed no change because live data had no broadcasts). Waiting for user acceptance before Done.
<!-- SECTION:NOTES:END -->

---
id: TASK-33
title: 'Hermes bridge: deliver replies to the chat thread the request was sent from'
status: To Do
assignee: []
created_date: '2026-10-16 15:58'
labels: []
dependencies: []
type: bug
ordinal: 32000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Symptom (2026-10-16): bridge kato was started by /hcom start in Telegram topic 30948 (session 20261016_130730_7d77e8ea). The user asked kato in topic 30963 (session 20261016_170603_d57b98ab); kato ran hcom send @hcom_dev --name kato --thread approval-command (event 170418). The reply 170424 (reply_to 170418, same thread) was injected into topic 30948, so the asking topic never saw it.

Root cause: src/hermes_plugin/__init__.py Bridge.hand_off injects every batch with the fixed origin stored at /hcom start (start_chat builds it from HERMES_SESSION_* incl. thread_id). Nothing records which chat/topic an outgoing send came from. Old fork behavior (the asking thread gets the reply) was not ported to orgoj-rebuild.

Scope: fixable in hcom alone (the hcom-owned plugin plus the binary); no Hermes change. inject_message(origin=...) already accepts any origin; the injection grant is per profile (hermes::grant_injection), not per thread.

Options:
A) Binary stamps the sender's Hermes session on the send event (needs HERMES_SESSION_ID/THREAD_ID in the gateway terminal subprocess env; unverified, ask hermes_dev). The plugin resolves reply_to/thread to that session and its origin_json in state.db. Robust, no command parsing.
B) Plugin only: GatewayHost.pre_tool_call runs in the per-chat context; on a terminal hcom send using a bridge name it records session_env() origin keyed by bridge+thread. Fragile (parses shell command strings, event id unknown at that time).
C) No code: /hcom start in each topic as a separate participant. Workaround only, splits identity.

Recommendation: A, fallback to the start origin when no stamp matches. Upstream file (Hermes integration is upstream 471f442): keep the hook minimal or offer upstream.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A message whose reply_to points to an event a bridge sent from another chat thread is injected into that thread, not the /hcom start thread
- [ ] #2 A message without reply_to in an hcom thread the bridge last sent to from another chat thread goes to that chat thread
- [ ] #3 Messages with no matching origin still go to the /hcom start origin
- [ ] #4 A delivery batch whose messages target different origins is injected per origin without loss or duplicate ack
- [ ] #5 Regression tests cover the reply, thread and fallback routing
<!-- AC:END -->

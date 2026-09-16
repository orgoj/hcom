# TODO

## Retry transient Codex SQLite startup failures

Codex CLI can exit before reaching `ready` when another Codex process briefly
holds `~/.codex/logs_2.sqlite` or `state_5.sqlite` (`database is locked` /
`another Codex process is using its local data`). Handle this in hcom's launch
readiness/failure path instead of changing Codex storage:

- Keep the standard shared `CODEX_HOME`, sessions, and SQLite locations.
- Retry only the recognized Codex lock-startup error, with a small bounded
  attempt count and backoff; never retry unrelated startup failures.
- Do not kill another Codex process. After retries are exhausted, report the
  actionable lock error and identify the holder when safely possible.
- Test clean start, concurrent start, tracked resume, fork, session switch, and
  nested child launch.

Upstream reports: openai/codex#30105 and openai/codex#35555. The discarded
per-instance SQLite prototype remains preserved on branch
`codex-sqlite-isolation` at commit `08f5d94` for reference only; do not use that
storage-isolation design as the fix.

## Recover Codex wake submission ignored during startup

A message to a newly launched Codex agent can remain unread even though the
message event, PTY, and screen parser are all healthy. Observed on `agent_coach`
with Codex 0.154.0: event `75506` was queued, `<hcom>` exclusively owned the
visible composer, but the initial Enter and all three retries left the text
unchanged. hcom then entered `tui:wake-unacknowledged`, where it deliberately
stops automatic submission and waits for hook progress, a session-switcher
cycle, or a process restart. Since Codex never submitted the prompt, hook
progress cannot occur and the delivery remains paused.

The likely race is that delivery starts in `Pending` and `evaluate_gate()` may
permit injection while the launch outcome is still pending. Codex's one-second
quiet-screen check is used by `launch_ready_observed()` but is not part of the
delivery gate, so a newly rendered composer can be treated as interactive too
early.

- Do not inject a startup wake until Codex launch readiness, including its
  quiet-screen period, has been observed. Keep normal delivery to an already
  running agent unaffected.
- If an exclusively hcom-owned wake survives all Enter attempts, retry later
  only after fresh evidence that the TUI became interactively ready. Bound the
  retry count/backoff and retain the unread message on exhaustion.
- Before every retry, revalidate exact prompt ownership, approval state, user
  activity, cursor, and live TUI layout. Never submit or clear mixed/user text.
- Preserve at-most-once behavior: advance the cursor only after hook
  acknowledgement or proof that no pending rows remain, and never inject a
  second wake while the first still owns the composer.
- Add regression coverage for a message queued during Codex autostart where
  early Enter presses are ignored and a later readiness-triggered retry
  succeeds. Also cover permanent failure, no duplicate delivery, user text,
  tracked resume, fork, session switch, and nested child launch.

## Recover Claude delivery stuck on an unpromoted live session

Observed on 2026-09-16 with `wdt_mail` and hcom `0.7.25-orgoj.4`. The instance
remained bound to primary Claude session
`184f8c8c-a153-4379-8ca3-b71b9e006d40`, while the live TUI continuously
reported hook session `2ee7b714-7ff2-4244-91b4-b61047a1fbcc`. Lineage
resolution identified the latter as owned by `wdt_mail`, but every hook was
rejected as `init_hook_context.unpromoted_lineage_rejected`. This was not a
temporary child-session transition: user prompts, tool hooks, polling, and
notifications continued under the rejected session throughout the work.

Request `81405` from `wdt_main` existed and remained unread. Both the native
injected `<hcom>` wake and a manually entered bare `<hcom>` reached the rejected
session, were blocked, and did not deliver the request. Native delivery then
entered `tui:wake-unacknowledged` and stopped automatic retries as designed.

A normal user prompt telling the agent to read its hcom messages did reach the
model despite hook initialization returning `no_instance`. The agent manually
found and completed request `81405`, then sent reply `81444` with
`--reply-to 81405`. Only that explicit reply advanced the delivery cursor from
`81038` to `81405`, cleared the pending row, and rearmed delivery. Hooks after
completion still reported the same unpromoted session, proving that no session
switch or promotion had repaired attribution.

Investigation for the next occurrence:

- Capture why Claude's visible, durable TUI session differs from the instance's
  recorded primary session. Do not assume that a background/child session will
  end or that a later prompt will return to the recorded primary.
- Determine whether a same-process, same-pane, same-owner lineage can be safely
  promoted using stronger evidence than inherited launch environment. Never
  auto-promote merely because the lineage resolves to the same name: that can
  misattribute a genuine child or switched session.
- Consider recovery after `unpromoted_lineage_rejected` that does not remain
  latched forever. A normal fallback prompt such as "read the pending hcom
  message" can wake the model, as this incident demonstrated, but it cannot be
  suppressed or converted to hook `additionalContext` when attribution fails;
  it enters model context and bypasses the normal delivery boundary. Treat that
  only as an explicit, carefully bounded fallback, not as equivalent to the
  invisible `<hcom>` sentinel.
- Add a regression fixture where the registered primary stays unchanged while
  all subsequent live-TUI hooks use a same-owner unpromoted session. Cover the
  failed automatic and manual bare wakes, preservation of the unread request,
  safe recovery, cursor advancement exactly once, a real child session, a user
  session switch, tracked resume, fork, and nested child launch.

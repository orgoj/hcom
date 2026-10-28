---
id: TASK-58
title: PTY launch aborts when delivery init exceeds 5s under hcom.db contention
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-10-28 07:41'
updated_date: '2026-10-28 09:55'
labels: []
dependencies: []
priority: high
type: bug
ordinal: 59000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Observed: agent_coach started from hcom-gui (07:05:36Z 2026-10-28, Herdr pane) died with 'Error: PTY run failed / Caused by: delivery start timed out'. The delivery thread (src/pty/shared.rs, initialize_delivery_components: HcomDb::open, NotifyServer, register_inject_port) did not finish within the 5s recv_timeout; on Unix src/pty/mod.rs bails on DeliveryStart::Pending and kills the session. At the same time the agent's SessionStart hook took 4.98s and other agents' hooks took 1-3s (wdt_main pre Edit 2.8s, sphere_dev_ansible_proweb_505 pre Bash 3.2s), pointing to hcom.db lock contention. Plan agreed with the user: first find what holds hcom.db so long (root cause), and as a safeguard raise the delivery init timeout.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The source of the multi-second hcom.db waits during launches is identified with log/DB evidence and recorded in the task notes
- [x] #2 The delivery init timeout is raised (e.g. 15-20s) so a launch survives a temporary DB stall, covered by a test
- [x] #3 A launch whose delivery init still times out reports the reason in hcom logs (delivery.init.timeout) and in the launch terminal's error output (PTY run failed: delivery start timed out)
- [x] #4 The identified contention is reduced: synchronous=NORMAL, verified by a commit-latency benchmark on a copy of the live hcom.db (FULL p50 10.3ms vs NORMAL 0.2ms); live verification after all agents restart is TASK-59
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Findings: (1) agent_coach SessionStart (bind_session_to_process -> BEGIN IMMEDIATE) and its delivery init both waited ~5s for the hcom.db write lock; delivery init hit its 5s recv_timeout and src/pty/mod.rs aborted the session. (2) 30-min /proc/locks sampler of the WAL write lock (shm byte 120): holds of 0.5-1.1s by 'hcom pty claude' delivery threads (single-statement heartbeat UPDATE etc.) and claude-mod-poll, in chains; at 08:14:05-13Z six back-to-back holds while wdt_main's pre Bash hook waited 5.25s. (3) Single-statement holds that long are commit fsyncs: ~/.hcom is on ZFS tank/home at 93% capacity, SQLite runs default synchronous=FULL (fsync on every WAL commit). (4) Benchmark on a VACUUM INTO copy, 5000 committed event inserts: FULL p50 10.3ms avg 13ms; NORMAL p50 0.2ms avg 1.9ms. (5) FTS5 insert triggers are not the cause (insert cost within the same numbers). Upstream has the same 5s delivery init timeout and Unix abort.

Pilot deploy of the working tree (0.7.30-orgoj.22) at 2026-10-28T09:02:57Z: a 30-min sampler still saw 12 holds of 0.5-0.8s, all by hcom pty processes started before the deploy (old binary, still synchronous=FULL); hooks after deploy were slower during a burst of 53 SessionStart hooks from instance 'knowledge' (09:29-09:36Z), so the load was not comparable. Live verification moved to TASK-59 because the user cannot restart agents now.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Root cause: WAL commit fsyncs (synchronous=FULL) on a slow/full ZFS held the hcom.db write lock 0.5-1.1s per single-statement write; chains of them made the delivery init miss its 5s timeout and the Unix PTY aborted the session. Fix: synchronous=NORMAL for read-write connections (apply_connection_pragmas, test OFF override kept), db::BUSY_TIMEOUT shared const; Unix Pending delivery init now waits the rest up to 30s (finish_pending_delivery) before aborting, Windows unchanged. Verified: copy-DB benchmark FULL p50 10.3ms vs NORMAL 0.2ms; unit tests for pragmas and pending paths; fmt, clippy, cargo test --bin hcom (only known-flaky idle_kill test, passes isolated), cli_smoke lifecycle/start_/agent_dry_run/pty; reviewed by agent_review.
<!-- SECTION:FINAL_SUMMARY:END -->

# Deferred upstream integration

Review date: 2026-10-03. Local baseline: `orgoj` at `4d9ab02`.
Decision: record the review now; attempt an upstream merge later, after explicit
authorization. No implementation or deployment is part of this review.

## Remote snapshot

All configured remotes were fetched with pruning. Upstream `main` is at
`2c5f343` (v0.7.27), with 31 commits added since the previous `e85efe4` review
and 69 commits since `fabb309`. Some older upstream changes were manually
ported locally, so ancestry counts do not measure missing functionality.

| Remote/ref | Tip | Assessment |
|---|---|---|
| upstream/main | `2c5f343` | First integration priority |
| michaelzag/main | `2a9bb47` | Selective reliability ports; do not merge wholesale |
| sirassss/main | `1b7c82d` | Rewritten history, current upstream plus ten fork commits |
| sirassss/feat/siras/develop | `4927391` | Additional Docker real-tool scenarios |
| atacolak/main | `e76d7ae` | Unchanged at this fetch |
| enferlain/main | `c2a1731` | Unchanged at this fetch; Hermes integration remains selective |
| tbelajonas71/main | `79ebde1` | Unchanged at this fetch |
| tbelajonas71/upstream/relay-reliability-20260926 | `8dadf46` | Relay, config command and wake source match upstream `566640c` |

## Recommended order

1. **Latest upstream.** Important changes include per-launch hooks (`6734893`)
   and nested hook isolation (`744088c`); unread cursor and inline delivery
   fixes (`201460f`, `d671eff`, `b4f9e0a`); Codex native approval/interrupt hooks
   (`8b99ab7`) and quiet-active delivery protection (`2f151c0`); Unix PTY
   backpressure fixes (`f8110bc`, `f21543e`); native resume diagnostics
   (`4161f4a`); missed relay event recovery, payload bounds and worker restart
   (`566640c`); orphan ownership (`e85efe4`) and process-identity cleanup
   (`c4e1727`).
2. **sirassss namespace-aware liveness (`6d8a442`).** Evaluate alongside
   upstream's new process cleanup. A PID invisible in a caller's namespace
   must not count as a dead host agent. This version extends upstream process
   identities rather than introducing the fork's older reconciler/schema.
   Cross-namespace `hcom kill` behavior still needs separate scrutiny.
3. **Michaelzag relay follow-ups (`2b64606`, `c86979d`, `c813c0f`).** Limit
   inbound draining by elapsed time and avoid unchanged short-id, capability
   and device-count writes. Include test follow-ups `5a17c59` and `efaee02`.
   Our baseline has a count-only drain and unconditional writes. Recompare
   after the upstream merge. The time budget checks between events and cannot
   bound a single slow event application.
4. **Michaelzag database contention handling.** Study `59dfde5`, `fb7a852`,
   `d26c3b4` and the corrective `4c0e75f` together. The final design keeps
   subscription database work in the transaction and defers TCP wakes until
   commit. Taking the initial post-commit fan-out patch alone would omit later
   fixes for event ordering and lost notifications. Treat this as a separately
   reviewed subsystem port.
5. **Small conditional sirassss ports.** From `d8b45e2`, consider the explicit
   `HERDR_AGENT` hint and absolute `env` capture. Pre-seeded env preservation is
   already local. Consider `4d36088` AGY readiness/banner and Codex spinner
   handling only with a matching symptom and comparison against our existing
   prompt protections.

## Compatibility and preserved fork behavior

Upstream `95caac5` removes `codex_sandbox_mode` and delegates permission policy
to native Codex configuration. Verify the effective policy before deployment;
do not assume old hcom settings still select it. Include subsequent writable
root fixes `061e7c9` and `2c5f343`, which preserve relative roots and table
overrides.

Already ported locally in `476b432`: upstream `e8fe709` (TUI reconnect after
reset), upstream `cf7127b` (retain killed reason during PTY cleanup), and
sirassss `776038e` (preserve pre-seeded env). Individual `git cherry` results
do not recognize this combined local patch.

Preserve catalog scope/privacy, named-agent launch and resume behavior, bundle
instructions and skills, terminal placement, AGY policy scope, and local
delivery recovery. For every upstream-rewritten fork file, inspect the fork's
own commits since the merge base and record each delta as retained or
deliberately dropped.

## Deferred features

Do not merge another fork wholesale. Defer sirassss's large TUI rewrite,
delivery escalation and Cursor/plugin packaging. Defer Michaelzag conductor
roles and message rerouting, fleet identity, compaction/context UX and its
CDN/release policy. Launch-timeout truth (`885c400`, `94026c2`) depends on its
process-truth architecture and needs a reproduced local failure first.

tbelajonas71's relay branch supplies no additional relay change beyond the
merged upstream `566640c`. Its older estate/desktop branches remain selective.
Enferlain Hermes `c2a1731` injects context while a turn is assembling; it is not
a substitute for Gateway idle wake and visible main-chat delivery.

## Future merge validation

Stage the eventual upstream merge on `orgoj-dev`, resolve conflicts and verify
the preserved fork behavior before advancing `orgoj`. Follow the repository's
test requirements and local release procedure. Include clean start, tracked
resume, fork, session switch and nested child launch checks for the new hook
and Codex configuration pipeline, plus PTY backpressure and relay coverage.

This review inspected source and history. Candidate changes have not been
executed or tested locally.

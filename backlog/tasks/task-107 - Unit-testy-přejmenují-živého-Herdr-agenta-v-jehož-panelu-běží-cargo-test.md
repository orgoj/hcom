---
id: TASK-107
title: Unit testy přejmenují živého Herdr agenta, v jehož panelu běží cargo test
status: Done
assignee: []
created_date: '2026-11-13T08:20:00Z'
updated_date: '2026-11-13T08:25:00Z'
labels:
  - bug
dependencies: []
ordinal: 215000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
cargo test --bin hcom spouští doručovací smyčku (hooks/claude.rs, delivery/grok.rs) s instancí 'nova'. HostLabel::resolve čte zděděné HERDR_ENV/HERDR_PANE_ID/HERDR_SOCKET_PATH z panelu vývojáře a pošle živému Herdr agent.rename 'nova'. Pak 'hcom agent attach hcom_dev' (Enter v tabulce dashboardu) selže, protože herdr agent focus hcom_dev nenajde agenta.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Unit testy nikdy neotevřou spojení na živý Herdr, ani se zděděnými HERDR_* proměnnými
- [x] #2 Regresní test: HostLabel::resolve se HERDR_ENV=1 v testu nemá backend
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Backend::resolve vrací v cfg(test) None, takže unit testy už nepřejmenují živého Herdr agenta (příčina selhání Enter/attach na hcom_dev v dashboardu). Regresní test resolve_never_reaches_a_live_herdr_in_unit_tests.
<!-- SECTION:FINAL_SUMMARY:END -->

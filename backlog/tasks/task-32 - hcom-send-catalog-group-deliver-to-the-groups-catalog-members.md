---
id: TASK-32
title: 'hcom send @<catalog group>: deliver to the group''s catalog members'
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-10-16 13:09'
updated_date: '2026-10-16 13:32'
labels: []
dependencies: []
type: feature
ordinal: 31000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
hcom send @cca_admin fails with 'No active agent for @cca_admin' although cca_admin is a catalog group in scope (hcom agent list @cca_admin: contimex_main, proweb_main, wdt_main). Catalog groups (fork feature) are resolved by 'hcom agent @group' and 'hcom kill @group', but send only resolves agent names, catalog aliases and running tag groups (tag-). Reported by bin (hcom #170061). Design open: precedence vs. an agent named like the group, autostart of every stopped member, catalog isolation (only members in scope).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 hcom send @<group> delivers to every in-scope catalog member of the group, autostarting stopped ones like a named send
- [x] #2 A running agent or catalog agent with the same name as the group keeps precedence
- [x] #3 Members hidden by catalog import scope are neither addressed nor revealed
- [x] #4 send help, README and skills/hcom-agent-messaging document @group; cli_smoke covers the send path
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
hcom send @<group> expands an explicit target to the group's in-scope catalog members (Catalogs::load), which route and autostart like named targets; a running or catalog agent of that name keeps precedence. Help (send, agent, top-level), README and skill updated. Verified: fmt, clippy, cargo test --bin hcom, cli_smoke send/group incl. targeted_send_to_catalog_group_starts_only_in_scope_members; agent_review OK.
<!-- SECTION:FINAL_SUMMARY:END -->

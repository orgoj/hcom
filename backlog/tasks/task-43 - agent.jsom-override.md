---
id: TASK-43
title: 'Catalog sidecar override: <name>.local.json next to every catalog'
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-10-23 13:35'
updated_date: '2026-10-23 14:42'
labels: []
dependencies: []
priority: medium
type: feature
ordinal: 42000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: the user needs to change anything in agents.json temporarily without leaving the tracked file dirty; a gitignored sidecar file holds the override.

Design (approved by agent_review): every catalog file hcom loads (global ~/.hcom/agents.json or HCOM_AGENTS_FILE, HCOM_AGENT_CATALOGS entries, imported catalogs, project .hcom/agents.json) has an optional sidecar `<stem>.local.json` in the same directory (non-.json path: `<name>.local.json`). At load time the sidecar is merged into its base catalog: defaults and agent entries merge like catalog layers (scalars replace; env, args, groups, inherit_env, tools merge), it may add agents and set disabled, cli_overrides merge per source (allowed only beside the global catalog). Precedence: base imports -> base -> sidecar. A sidecar may not declare imports (error naming the sidecar). The sidecar has exactly the scope of its base file (import agents filter, project scope). It is listed in `hcom agent show` sources after its base; `agent list --json` source stays the base label, so hcom-gui needs no change. The global catalog is loaded when agents.json, its sidecar or agents/ bundles exist.
Out of scope: hcom writing or managing .gitignore; a CLI to edit the sidecar.
Risks: catalog isolation (AGENTS.md) - no listing or error may reveal hidden agents through a sidecar.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A global sidecar overrides a scalar, merges env and appends args of a base agent, overrides defaults applying to base agents, adds an agent and disables a base agent
- [x] #2 A global sidecar's cli_overrides merge per source; a non-global sidecar with cli_overrides, a sidecar with imports and an invalid sidecar JSON each fail with an error naming the sidecar path
- [x] #3 hcom agent show lists the sidecar in sources after its base; agent list --json keeps the base label as source for both an overridden and a sidecar-added agent
- [x] #4 agents.local.json without agents.json in the global hcom directory is loaded
- [x] #5 Without sidecars behavior is unchanged (existing tests pass); cli_smoke tests cover every case above; agent help, README.md, docs and skills/hcom-agent-messaging describe the sidecar
- [x] #6 Sidecars next to an HCOM_AGENT_CATALOGS file, an imported catalog and a project .hcom/agents.json override their base's agents; an agent added by an imported catalog's sidecar is filtered out by the import's agents list, and a project sidecar's added agent is not addressable outside the project
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. load_catalog_file: parse the base and the sidecar with the same validation (parse_catalog), reject sidecar imports, merge defaults/agents/cli_overrides into the base, then run the single agents/ bundle scan on the merged catalog; record the sidecar path in CatalogFile.
2. Catalogs::paths lists the sidecar after its base; global gate also checks the sidecar.
3. cli_smoke cases per criterion; help text, README, docs, skill.
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
load_catalog_file merges <stem>.local.json into its catalog (parse_catalog split out; sidecar imports rejected; cli_overrides global-only; single bundle scan after merge); sidecar listed in agent show sources; global loaded when only the sidecar exists. Docs: agent help, top-level help, README, skill. Verified with 3 new cli_smoke tests plus agent/catalog/send/events suites, fmt, clippy, cargo test --bin hcom; reviewed by agent_review.
<!-- SECTION:FINAL_SUMMARY:END -->

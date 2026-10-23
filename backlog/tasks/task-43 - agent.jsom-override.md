---
id: TASK-43
title: 'Catalog sidecar override: <name>.local.json next to every catalog'
status: To Do
assignee: []
created_date: '2026-10-23 13:35'
updated_date: '2026-10-23 14:07'
labels: []
dependencies: []
priority: medium
type: feature
ordinal: 42000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: the user needs to change anything in agents.json temporarily without leaving the tracked file dirty; a gitignored sidecar file holds the override.

In scope: every catalog file hcom loads (global ~/.hcom/agents.json or HCOM_AGENTS_FILE, HCOM_EXTRA_CATALOGS entries, imported catalogs, project .hcom/agents.json) gets an optional sidecar `<stem>.local.json` in the same directory, loaded right after its base file as one more catalog layer with the same schema and merge semantics (scalars replace, env/args merge, may add agents, set disabled, defaults, imports; cli_overrides only beside the global catalog). The sidecar has the scope of its base file: a project sidecar stays project-scoped and an imported catalog's sidecar is filtered by the same import `agents` list. The sidecar appears in `hcom agent show` sources and `agent list --json` source; hcom-gui handles the new source labels.
Out of scope: hcom writing or managing .gitignore (the user maintains it); a CLI to edit the sidecar.
Risks: catalog isolation (AGENTS.md) - no listing or error may reveal hidden agents through a sidecar; upstream mergeability - keep the hook inside the existing catalog loader.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A sidecar <stem>.local.json next to the global catalog, an HCOM_EXTRA_CATALOGS file, an imported catalog and a project .hcom/agents.json each overrides fields of its base file's agents, and env/args merge with the base values
- [ ] #2 A sidecar may add an agent and disable an agent; a project sidecar's agents are addressable only where its project catalog is in scope, and an imported catalog's sidecar adds nothing outside the import's agents list
- [ ] #3 Without sidecars, catalog resolution and output are unchanged; an invalid sidecar fails with the same kind of error naming the sidecar path as an invalid base catalog
- [ ] #4 hcom agent show lists the sidecar in sources and agent list --json reports it in source; hcom-gui catalog_files/catalog_cwd/catalog_label handle the sidecar label
- [ ] #5 cli_smoke tests cover override, env merge, added agent, disabled agent, import filtering and project scope; agent help (src/commands/help.rs / agent.rs help text), README.md, docs and skills/hcom-agent-messaging describe the sidecar
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. In load_catalog_tree_with_mode / load_with_mode, after each base CatalogFile append the sidecar CatalogFile when present, inheriting its label/base/import filter.
2. Give the sidecar a distinct source label; adjust agent show/list output and hcom-gui parsing.
3. cli_smoke cases per criterion, help/README/docs/skill updates.
<!-- SECTION:PLAN:END -->

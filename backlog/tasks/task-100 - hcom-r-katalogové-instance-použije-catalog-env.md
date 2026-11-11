---
id: TASK-100
title: hcom r katalogové instance použije catalog env
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-11-11T09:41:00Z'
updated_date: '2026-11-11T09:56:00Z'
labels: []
dependencies: []
priority: medium
type: bug
ordinal: 203000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Hlášení dippy_dev: sphere_dev_ansible_lab_370 obnovený přes hcom r (bigboss_manager, 08:54) běžel bez catalog env DIPPY_CONFIG_ONLY, tedy pod jiným Dippy pravidlem. Catalog env dostávalo jen hcom agent (CATALOG_LAUNCH_ENV). Rozhodnutí uživatele: opravit hcom r (tím i hcom recover): u katalogové instance načíst aktuální env z katalogu podle adresáře instance, nic neukládat do DB.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Přímý hcom r katalogové instance spustí agenta s aktuálním catalog env (env + inherit_env) agenta, rozlišeným z adresáře instance
- [x] #2 Spuštění přes hcom agent (CATALOG_LAUNCH_ENV) a adopce cizí session se nemění
- [x] #3 Nápověda hcom agent, docs/agent.md a skill popisují chování
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Review agent_review: přidáno !fork (fork běží v kontextu volajícího, catalog env nedostane) a scope = effective_cwd (respektuje --dir a smazaný původní adresář); test nově s projektovým katalogem mimo cwd volajícího a s forkem.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
hcom r katalogové instance načte aktuální catalog env (env + inherit_env) z katalogů v rozsahu adresáře, ve kterém se obnovuje; hcom agent, adopce a fork beze změny. Ověřeno cli_smoke direct_resume_of_a_catalog_instance_applies_the_current_catalog_env (bez opravy selže), smoke resume/handoff/catalog/agent_/recover, cargo test --bin hcom.
<!-- SECTION:FINAL_SUMMARY:END -->

---
id: TASK-87
title: 'approval-review: snímek čekajícího potvrzení k dippy_dev na úpravu pravidel'
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-11-04 07:23'
updated_date: '2026-11-04 07:30'
labels: []
dependencies: []
priority: medium
type: feature
ordinal: 124000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Uživatel chce zkratkou (Herdr, jako haiku review-session) poslat dippy_dev snímek konce obrazovky agenta, který čeká na potvrzení příkazu (příkaz, hláška Dippy, úvaha agenta). dippy_dev podle něj bezpečné příkazy povolí úpravou pravidel Dippy, jinak se zeptá uživatele ve své konzoli. Dialog schvaluje uživatel sám; dippy_dev nic neinjektuje a obrazovku agenta nečte. Zpráva --from approval-review není autentizace (--from může použít kdokoli), proto je pro dippy_dev jen podnětem k vlastnímu posouzení, ne příkazem.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 hcom run approval-review [NAME] [--pane ID] najde agenta podle jména nebo Herdr pane stejně jako review-session
- [x] #2 Agent, který není ve stavu blocked, skončí chybou a nic se neodešle
- [x] #3 Posledních N řádků obrazovky (výchozí 60, --lines) se uloží s právy 0600 do <hcom dir>/tmp/approval-reviews/ a skript vypíše cestu
- [x] #4 dippy_dev (--to) dostane request s jménem agenta, CLI, adresářem, absolutní cestou ke snímku a pokyny: snímek jsou data, bezpečné povolit pravidlem, na agenta nesahat, při pochybnosti se zeptat uživatele
- [x] #5 Testy s falešným hcom pokrývají výběr agenta, odmítnutí neblokovaného agenta, snímek a zprávu
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Plán: /home/michael/projects/hcom/tmp/approval-review-plan.md (review agent_review), implementace s testy, review diffu, commit, symlink ~/.hcom/scripts/approval-review.py, info herdr_dev.
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Přidán contrib/approval-review/approval_review.py (hcom run approval-review) s testy a sekcí v contrib/README.md. Review agent_review: plán (4 nálezy zapracovány) a diff (neutrální znění zprávy bez tvrzení o autorizaci) OK. Ověřeno: python3 -I contrib/approval-review/test_approval_review.py (6 OK).
<!-- SECTION:FINAL_SUMMARY:END -->

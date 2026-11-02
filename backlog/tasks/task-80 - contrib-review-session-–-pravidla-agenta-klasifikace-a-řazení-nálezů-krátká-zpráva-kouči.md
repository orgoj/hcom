---
id: TASK-80
title: >-
  contrib: review-session – pravidla agenta, klasifikace a řazení nálezů, krátká
  zpráva kouči
status: Done
assignee:
  - '@hcom_dev'
created_date: '2026-11-02 14:52'
updated_date: '2026-11-02 15:27'
labels: []
dependencies: []
priority: medium
type: feature
ordinal: 108000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Požadavek uživatele přes agent_coach (#276798) po reportu ~/.hcom/tmp/session-reviews/herdr_dev-20261102-154413-701316.md: Haiku navrhlo pravidlo, které už globálně existuje, zpráva kouči nese celý report, triviální nálezy mají stejnou váhu jako závažné a závěr není na začátku.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Reviewer dostane cesty k efektivním pravidlům agenta: výstup hcom agent show (globální system prompt + SOUL.md) uložený vedle reportu, NOTES.md bundlu, AGENTS.md projektu a globální instrukce CLI (claude ~/.claude/CLAUDE.md, codex ~/.codex/AGENTS.md), jen existující
- [x] #2 Každý nález má závažnost, třídu (porušení existujícího pravidla s citací / chybějící pravidlo / chybějící znalost) a cíl (NOTES, SOUL, skill, repo, globální prompt); prompt zakazuje navrhovat pravidlo, které už v pravidlech je
- [x] #3 Nálezy jsou seřazené podle závažnosti; triviální (povolené pravidly agenta, hedging, který reviewer sám uznal za správný, běžná spotřeba tokenů) vypadnou nebo mají jeden řádek v sekci Minor
- [x] #4 Report začíná sekcí Conclusion
- [x] #5 Zpráva kouči nese jen cestu k reportu, počet nálezů a jeden řádek s nejzávažnějším nálezem, nikdy celý report
- [x] #6 Testy v contrib/review-session/test_review_session.py pokrývají cesty k pravidlům, agenta bez katalogu a formát zprávy
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
review_session.py: prompt lists the agent's rules (hcom agent show output saved as <stem>-rules.txt, bundle NOTES.md, project AGENTS.md, CLI global instructions); failed lookup vs no catalog entry distinguished; catalog selection env kept for the show call; findings classified (violation/missing rule/missing knowledge) with target, ranked by severity, trivial under Minor; report starts with Conclusion; coach message carries only count, top finding and path. Verified: 14 tests in test_review_session.py, live rules_section on herdr_dev; agent_review OK on plan and diff.
<!-- SECTION:FINAL_SUMMARY:END -->

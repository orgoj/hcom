---
id: TASK-80
title: >-
  contrib: review-session – pravidla agenta, klasifikace a řazení nálezů, krátká
  zpráva kouči
status: In Progress
assignee:
  - '@hcom_dev'
created_date: '2026-11-02 14:52'
updated_date: '2026-11-02 14:52'
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
- [ ] #1 Reviewer dostane cesty k efektivním pravidlům agenta: výstup hcom agent show (globální system prompt + SOUL.md) uložený vedle reportu, NOTES.md bundlu, AGENTS.md projektu a globální instrukce CLI (claude ~/.claude/CLAUDE.md, codex ~/.codex/AGENTS.md), jen existující
- [ ] #2 Každý nález má závažnost, třídu (porušení existujícího pravidla s citací / chybějící pravidlo / chybějící znalost) a cíl (NOTES, SOUL, skill, repo, globální prompt); prompt zakazuje navrhovat pravidlo, které už v pravidlech je
- [ ] #3 Nálezy jsou seřazené podle závažnosti; triviální (povolené pravidly agenta, hedging, který reviewer sám uznal za správný, běžná spotřeba tokenů) vypadnou nebo mají jeden řádek v sekci Minor
- [ ] #4 Report začíná sekcí Conclusion
- [ ] #5 Zpráva kouči nese jen cestu k reportu, počet nálezů a jeden řádek s nejzávažnějším nálezem, nikdy celý report
- [ ] #6 Testy v contrib/review-session/test_review_session.py pokrývají cesty k pravidlům, agenta bez katalogu a formát zprávy
<!-- AC:END -->

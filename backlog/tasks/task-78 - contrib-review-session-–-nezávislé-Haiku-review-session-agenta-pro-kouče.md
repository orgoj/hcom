---
id: TASK-78
title: 'contrib: review-session – nezávislé Haiku review session agenta pro kouče'
status: To Do
assignee: []
created_date: '2026-11-01 17:00'
labels: []
dependencies: []
priority: medium
type: feature
ordinal: 90000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Cíl: review běžícího (hotového, ještě neukončeného) agenta na jeden příkaz/klik, levně (Haiku), nezávislým pohledem zvenku; report dostane kouč, který podle něj upraví agenta. Doplňuje skilly session-review / reviewing-sessions-with-coach, které stojí na sebehodnocení agenta.

Rozsah: skript v contrib/ (ne v binárce), do ~/.hcom/scripts/ nalinkovaný, spouštění 'hcom run review-session NAME'. Skript zjistí metadata (hcom list NAME --json: transcript_path, session_id, tool, directory), spustí 'claude -p --model haiku' s pevným review promptem; Haiku si sám čte 'hcom transcript' (rozsahy, --detailed, search) a 'hcom events --agent NAME', najde problémy a sepíše report o tom, jak agent pracoval. Oprávnění příkazů Haiku řeší Dippy, skript je neomezuje vlastním allowlistem. Report jde do souboru; skript pošle koučovi zprávu: pevný coach prompt (jak report zpracovat, je součástí skriptu) + obsah reportu ze souboru. Kouč: argument skriptu, jinak výchozí agent_coach.

Mimo rozsah: automatické spouštění při idle, vzdálení agenti, změny binárky hcom, klikací tlačítko v herdr/hcom-gui (samostatný task).

Rizika: zděděné HCOM_* env by claude -p / jeho hcom příkazy vydávalo za reviewovaného agenta; dlouhé transcripty se nevejdou do kontextu Haiku (proto čte po částech); Haiku může vymýšlet problémy (coach prompt je označí jako podněty).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 contrib/ obsahuje skript review-session; po nalinkování do ~/.hcom/scripts/ ho spustí 'hcom run review-session NAME' a '--help' popisuje použití
- [ ] #2 Skript pro NAME spustí claude -p s modelem haiku a pevným review promptem obsahujícím identifikaci session (jméno, tool, session_id, transcript_path); funguje i pro agenta, který ještě běží
- [ ] #3 Proces claude -p ani jeho hcom příkazy nedědí HCOM_* identitu reviewovaného ani volajícího agenta (ověřeno testem)
- [ ] #4 Report se uloží do souboru a koučovi přijde jedna hcom zpráva: coach prompt ze skriptu následovaný obsahem reportu
- [ ] #5 Kouč se bere z argumentu skriptu, bez něj je výchozí agent_coach
- [ ] #6 Skript má vlastní testy v contrib/ (bez volání skutečného claude), dokumentaci v contrib/README.md
<!-- AC:END -->

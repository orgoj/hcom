#!/usr/bin/env python3
# Independent Haiku review of an agent's session, sent to a coach.
"""Independent Haiku review of an agent's session, sent to a coach.

Usage: hcom run review-session [NAME] [--pane ID] [--coach NAME] [--model MODEL]

Reviews the session of the running hcom agent NAME with `claude -p` (default model haiku)
and sends the report to the coach (default agent_coach). Without NAME the agent is the one
launched in the Herdr pane --pane, or else $HERDR_PANE_ID (ignored when NAME is given). The report is saved under
<hcom dir>/tmp/session-reviews/ and its path printed; the coach gets only the finding count, the
top finding and that path.
"""

import argparse
import json
import os
import re
import subprocess
import sys
import time

REVIEW_PROMPT = """\
You are an independent reviewer of the hcom agent session below. You only read; never
send hcom messages, inject into terminals, kill or start agents, or change any file.

Agent: {name}
CLI: {tool}
Session id: {session_id}
Transcript file: {transcript_path}
Working directory: {directory}

Read the session with hcom (it also works while the agent is still running):
- `hcom transcript {name} --last 20` to start; ranges `hcom transcript {name} N-M`,
  `--full` for complete replies, `--detailed` for tool calls, errors and token usage.
- `hcom transcript search "pattern"` to find corrections, errors or repeated attempts.
- `hcom events --agent {name} --last 50` for status, messages and lifecycle.
Read long transcripts in ranges; never load the whole file at once.

Rules the agent had (read them all before judging; the catalog part is resolved now, not
recorded at launch):
{rules}

Find how the agent worked: errors, user corrections, ignored or misread instructions,
wasted effort (loops, repeated failed commands, needless reading), token waste, and
practices worth keeping. Cite exchange numbers as evidence. Mark anything you are unsure
of as uncertain; do not invent problems.

Classify each problem against the rules above:
- violation: an existing rule covers it; quote the rule and name its file.
- missing rule: no rule covers it. Never propose a rule that already exists; that is a violation.
- missing knowledge: a fact about this machine, tool or project the agent lacked.
Target of the fix: NOTES (bundle NOTES.md), SOUL (agent instructions), skill, repo (AGENTS.md,
docs) or global prompt.
Rank problems most severe first. Drop trivial items, or give each one line under Minor:
anything the agent's rules allow (e.g. an allowed `sed -i`), hedging you yourself judge correct,
normal token usage such as cache reads.

Output only the report in Markdown:
# Session review: {name}
## Conclusion (1-3 sentences: overall verdict and the most severe problem)
## Problems
1. **[high|medium|low] <one-line title>** then: what happened, evidence, class, target, fix.
## Minor (one line each, or "none")
## Good practices
## Uncertain
Nothing after the Uncertain section: no STATUS block, no closing remarks.
"""

COACH_MESSAGE = """\
Independent session review of agent {name} by {model} (cheap model, outside view): {summary}
Treat the findings as hints, not verdicts: verify each against the transcript before acting.
Report: {report}
"""

# CLI-wide instruction files the reviewed agent's CLI loads on its own.
CLI_GLOBAL_INSTRUCTIONS = {
    "claude": lambda: os.path.expanduser("~/.claude/CLAUDE.md"),
    "codex": lambda: os.path.join(os.environ.get("CODEX_HOME") or os.path.expanduser("~/.codex"),
                                  "AGENTS.md"),
}
CATALOG_ENV = {"HCOM_AGENTS_FILE", "HCOM_AGENT_CATALOGS"}  # catalog selection, not identity
PROBLEM_ITEM = re.compile(r"^\d+\.\s+(.*)$")


# Caller CLI session markers, as hcom's launcher clears them for children (tool_detection.rs).
CALLER_MARKERS = {"CLAUDECODE", "CLAUDE_ENV_FILE", "CLAUDE_CODE_SESSION_ID",
                  "CODEX_THREAD_ID", "CODEX_SESSION_ID"}
TIMEOUT = 1800  # seconds for the claude -p review
STATUS_BLOCK = re.compile(r"\n+STATUS: .*(\n(Done|Issues|Next):.*)*\s*\Z")


def hcom_env(keep=frozenset()):
    """Environment without the caller's hcom identity (HCOM_DIR and `keep` stay) and without
    the caller CLI's session markers."""
    return {k: v for k, v in os.environ.items()
            if (not k.startswith("HCOM_") or k == "HCOM_DIR" or k in keep)
            and k not in CALLER_MARKERS}


def hcom(*args):
    out = subprocess.run(["hcom", *args], env=hcom_env(), capture_output=True, text=True)
    if out.returncode:
        sys.exit(f"review-session: hcom {' '.join(args)} failed: {out.stderr.strip()}")
    return out.stdout


def find_agent(name, pane):
    """The agent NAME, else the one agent launched in Herdr pane `pane` (NAME wins over pane)."""
    agents = json.loads(hcom("list", "--json"))
    if name:
        found = [a for a in agents if a.get("name") == name]
    else:
        found = [a for a in agents
                 if (a.get("launch_context") or {}).get("terminal_preset_effective") == "herdr"
                 and (a.get("launch_context") or {}).get("pane_id") == pane]
    if len(found) > 1:
        sys.exit(f"review-session: several agents in pane {pane}: "
                 f"{', '.join(a['name'] for a in found)}; give NAME")
    if not found:
        sys.exit(f"review-session: no running agent {name or 'in pane ' + pane}")
    return found[0]


def write_private(path, text):
    # Owner-only: reports and rules quote the transcript and instructions.
    with os.fdopen(os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600), "w") as f:
        f.write(text)


def rules_section(a, rules_file):
    """Prompt lines naming the rule sources of agent `a`; writes `hcom agent show` to rules_file."""
    lines = []
    directory = a.get("directory") or ""
    catalog = (a.get("launch_context") or {}).get("agent_name")
    if not catalog:
        lines.append("- No catalog entry (ad hoc agent): no hcom system prompt or bundle.")
    else:
        try:
            out = subprocess.run(["hcom", "agent", "show", catalog], env=hcom_env(CATALOG_ENV),
                                 cwd=directory or None, capture_output=True, text=True)
            error = out.stderr.strip().splitlines()[:1] if out.returncode else None
        except OSError as e:
            out, error = None, [str(e)]
        if error is not None:
            lines.append(f"- Catalog rules could not be retrieved ({': '.join(error) or 'error'}); "
                         "mark every conclusion that depends on them (violation or missing rule) "
                         "uncertain.")
        else:
            write_private(rules_file, out.stdout)
            lines.append(f"- {rules_file}: `hcom agent show {catalog}`, the effective hcom system "
                         "prompt including the agent's SOUL.md instructions.")
            bundle = re.search(r"^bundle:[ \t]+(.+?)[ \t]*$", out.stdout, re.M)
            notes = os.path.join(bundle.group(1), "NOTES.md") if bundle else ""
            if notes and os.path.isfile(notes):
                lines.append(f"- {notes}: the agent's bundle notes.")
    for path, what in ((os.path.join(directory, "AGENTS.md") if directory else "", "project rules"),
                       (CLI_GLOBAL_INSTRUCTIONS.get(a.get("tool"), lambda: "")(),
                        f"{a.get('tool')} global instructions")):
        if path and os.path.isfile(path):
            lines.append(f"- {path}: {what}.")
    return "\n".join(lines)


def findings_summary(text):
    """'N findings; top: <title of item 1>' from the report's Problems section."""
    section = re.search(r"^## Problems[^\n]*\n(.*?)(?=^## |\Z)", text, re.M | re.S)
    items = [m.group(1) for line in (section.group(1).splitlines() if section else [])
             if (m := PROBLEM_ITEM.match(line))]
    if not items:
        return "no findings."
    top = items[0].replace("**", "").strip()
    top = top if len(top) <= 200 else top[:199] + "…"
    return f"{len(items)} finding{'s' if len(items) > 1 else ''}; top: {top}"


def main(argv):
    p = argparse.ArgumentParser(prog="hcom run review-session", description=__doc__,
                                formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("agent", nargs="?", help="agent name (default: the agent in the Herdr pane)")
    p.add_argument("--pane", default=os.environ.get("HERDR_PANE_ID"), help="Herdr pane id")
    p.add_argument("--coach", default="agent_coach", help="recipient (default: agent_coach)")
    p.add_argument("--model", default="haiku", help="claude model (default: haiku)")
    p.add_argument("--name", help=argparse.SUPPRESS)  # caller identity passed by hcom run; unused
    args = p.parse_args(argv)
    if not args.agent and not args.pane:
        p.error("give NAME or --pane (no HERDR_PANE_ID)")

    a = find_agent(args.agent, args.pane)
    name = a["name"]
    outdir = os.path.join(os.environ.get("HCOM_DIR") or os.path.expanduser("~/.hcom"),
                          "tmp", "session-reviews")
    os.makedirs(outdir, exist_ok=True)
    stem = os.path.join(outdir, f"{name}-{time.strftime('%Y%m%d-%H%M%S')}-{os.getpid()}")
    report = stem + ".md"
    prompt = REVIEW_PROMPT.format(name=name, rules=rules_section(a, stem + "-rules.txt"),
                                  **{k: a.get(k) or "" for k in
                                     ("tool", "session_id", "transcript_path", "directory")})

    print(f"review-session: reviewing {name} with {args.model}...", file=sys.stderr)
    try:
        run = subprocess.run(["claude", "-p", "--model", args.model, prompt], env=hcom_env(),
                             cwd=outdir, capture_output=True, text=True, timeout=TIMEOUT)
    except subprocess.TimeoutExpired:
        sys.exit(f"review-session: claude timed out after {TIMEOUT}s")
    if run.returncode or not run.stdout.strip():
        sys.exit(f"review-session: claude failed ({run.returncode}): {run.stderr.strip()}")
    text = STATUS_BLOCK.sub("", run.stdout).rstrip() + "\n"  # trailing turn-end block, not report
    write_private(report, text)

    message = COACH_MESSAGE.format(name=name, model=args.model, report=report,
                                   summary=findings_summary(text))
    out = subprocess.run(["hcom", "send", f"@{args.coach}", "--from", "session-review",
                          "--intent", "inform"], input=message, env=hcom_env(),
                         capture_output=True, text=True)
    if out.returncode:
        sys.exit(f"review-session: report saved to {report}, send failed: {out.stderr.strip()}")
    print(report)


if __name__ == "__main__":
    main(sys.argv[1:])

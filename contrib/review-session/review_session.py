#!/usr/bin/env python3
# Independent Haiku review of an agent's session, sent to a coach.
"""Independent Haiku review of an agent's session, sent to a coach.

Usage: hcom run review-session [NAME] [--pane ID] [--coach NAME] [--model MODEL]

Reviews the session of the running hcom agent NAME with `claude -p` (default model haiku)
and sends the report to the coach (default agent_coach). Without NAME the agent is the one
launched in the Herdr pane --pane, or else $HERDR_PANE_ID (ignored when NAME is given). The report is saved under
<hcom dir>/tmp/session-reviews/ and its path printed.
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
- The project's instructions (`AGENTS.md` in the working directory) to judge rule compliance.
Read long transcripts in ranges; never load the whole file at once.

Find how the agent worked: errors, user corrections, ignored or misread instructions,
wasted effort (loops, repeated failed commands, needless reading), token waste, and
practices worth keeping. Cite exchange numbers as evidence. Mark anything you are unsure
of as uncertain; do not invent problems.

Output only the report in Markdown:
# Session review: {name}
## Summary (2-4 sentences)
## Problems (each: what happened, evidence, suggested instruction/skill change)
## Good practices
## Uncertain
Nothing after the Uncertain section: no STATUS block, no closing remarks.
"""

COACH_PROMPT = """\
Independent session review of agent {name} by {model} (cheap model, outside view).
Treat the findings as hints, not verdicts: verify each against the transcript before acting.
For confirmed problems, improve the agent's instructions, skills or catalog entry the usual
way. Report file: {report}

"""


# Caller CLI session markers, as hcom's launcher clears them for children (tool_detection.rs).
CALLER_MARKERS = {"CLAUDECODE", "CLAUDE_ENV_FILE", "CLAUDE_CODE_SESSION_ID",
                  "CODEX_THREAD_ID", "CODEX_SESSION_ID"}
TIMEOUT = 1800  # seconds for the claude -p review
STATUS_BLOCK = re.compile(r"\n+STATUS: .*(\n(Done|Issues|Next):.*)*\s*\Z")


def hcom_env():
    """Environment without the caller's hcom identity (HCOM_DIR stays: the data directory)
    and without the caller CLI's session markers."""
    return {k: v for k, v in os.environ.items()
            if (not k.startswith("HCOM_") or k == "HCOM_DIR") and k not in CALLER_MARKERS}


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
    prompt = REVIEW_PROMPT.format(name=name, **{k: a.get(k) or "" for k in
                                  ("tool", "session_id", "transcript_path", "directory")})

    outdir = os.path.join(os.environ.get("HCOM_DIR") or os.path.expanduser("~/.hcom"),
                          "tmp", "session-reviews")
    os.makedirs(outdir, exist_ok=True)
    report = os.path.join(outdir, f"{name}-{time.strftime('%Y%m%d-%H%M%S')}-{os.getpid()}.md")

    print(f"review-session: reviewing {name} with {args.model}...", file=sys.stderr)
    try:
        run = subprocess.run(["claude", "-p", "--model", args.model, prompt], env=hcom_env(),
                             cwd=outdir, capture_output=True, text=True, timeout=TIMEOUT)
    except subprocess.TimeoutExpired:
        sys.exit(f"review-session: claude timed out after {TIMEOUT}s")
    if run.returncode or not run.stdout.strip():
        sys.exit(f"review-session: claude failed ({run.returncode}): {run.stderr.strip()}")
    text = STATUS_BLOCK.sub("", run.stdout).rstrip() + "\n"  # trailing turn-end block, not report
    # Owner-only: the report quotes the transcript.
    with os.fdopen(os.open(report, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600), "w") as f:
        f.write(text)

    message = COACH_PROMPT.format(name=name, model=args.model, report=report) + text
    out = subprocess.run(["hcom", "send", f"@{args.coach}", "--from", "session-review",
                          "--intent", "inform"], input=message, env=hcom_env(),
                         capture_output=True, text=True)
    if out.returncode:
        sys.exit(f"review-session: report saved to {report}, send failed: {out.stderr.strip()}")
    print(report)


if __name__ == "__main__":
    main(sys.argv[1:])

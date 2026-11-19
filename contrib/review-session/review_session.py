#!/usr/bin/env python3
# Independent Haiku review of an agent's session, sent to a coach.
"""Independent Haiku review of an agent's session, sent to a coach.

Usage: hcom run review-session [NAME] [--pane ID] [--coach NAME] [--model MODEL]
       hcom run review-session NAME --day YYYY-MM-DD [--cli claude|agy] [--coach NAME] [--model MODEL]

Reviews the session of the running hcom agent NAME with `claude -p` (default model haiku)
and sends the report to the coach (default agent_coach). Without NAME the agent is the one
launched in the Herdr pane --pane, or else $HERDR_PANE_ID (ignored when NAME is given). The report is saved under
<hcom dir>/tmp/session-reviews/ and its path printed; the coach gets only the finding count, the
top finding and that path.

--day reviews a whole local day instead: every session of NAME and its clones (NAME_*), stopped
or running, that has exchanges that day. The script writes one dump next to the report (token
totals from the transcripts' usage records, a timeline of prompts, messages, blocked approvals and
long gaps, the agent's rules, NOTES and MEMORY, and the detailed transcripts) and the model reads
it. --cli agy runs the analysis with `agy -p` (default model gemini-4-neon-medium, 1M context).
"""

import argparse
import glob
import json
import os
import re
import subprocess
import sys
import time
from datetime import datetime, timedelta, timezone

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

DAY_PROMPT = """\
You are an independent reviewer of one day of work of the hcom agent {name} and its clones
on {day}. You only read; never run commands that change anything, send messages, or edit files.

Everything you need is in the file {dump} in the current directory: token totals, a timeline
(prompts, hcom messages, blocked approvals, long gaps), the agent's rules and memory (SOUL.md is
part of the hcom system prompt shown by `hcom agent show`), and the detailed transcripts of
these sessions with tool I/O and errors:
{sessions}
Read the whole file, in ranges if it is large.

Goal: explain why the work took so long and cost so many tokens, so the agent's instructions can
be improved. Find where it lost time and tokens: retries and repeated failed commands, blocked
approvals, needless reads and searches, oversized outputs, waiting on other agents or the user,
long gaps, loops, rework after corrections. Quantify with times and token numbers from the dump.
For each loss name the line in SOUL.md, NOTES.md or MEMORY that causes it (quote it, name the
file), or the rule or knowledge that is missing and where it belongs (SOUL, NOTES, MEMORY, skill,
repo AGENTS.md/docs, global prompt). Never propose a rule that already exists; that is a
violation: quote the rule. Mark anything you are unsure of as uncertain; do not invent problems.

Output only the report in Markdown:
# Day review: {name} {day}
## Conclusion (1-3 sentences: where most time and tokens went)
## Numbers (sessions, exchanges, tokens, longest gaps, blocked approvals, waits)
## Problems
1. **[high|medium|low] <one-line title>** then: what happened, evidence (session, time,
   exchange), cost (time, tokens), class (violation|missing rule|missing knowledge), target,
   proposed change (the exact line to add, change or delete).
## Proposed instruction changes (grouped by file)
## Good practices
## Uncertain
Nothing after the Uncertain section: no STATUS block, no closing remarks.
"""

DAY_COACH_MESSAGE = """\
Day review of agent {name} and clones for {day} ({count} sessions) by {model}: {summary}
Treat the findings as hints, not verdicts: verify each against the transcript before acting.
Report: {report}
Dump (tokens, timeline, transcripts): {dump}
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
DAY_TIMEOUT = 3600  # seconds for a --day review
GAP = 600  # seconds without any event that the timeline reports as a gap
DEFAULT_MODEL = {"claude": "haiku", "agy": "gemini-4-neon-medium"}
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


def parse_ts(ts):
    t = datetime.fromisoformat(ts)
    return t if t.tzinfo else t.replace(tzinfo=timezone.utc)  # hcom event ts are UTC


def iso(t):
    return t.astimezone(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def events(*args):
    out = hcom("events", "--last", "1000000", *args)
    return [json.loads(line) for line in out.splitlines() if line.strip()]


def day_sessions(name, start, end):
    """Sessions of NAME and NAME_* that may have exchanges in [start, end): stopped snapshots
    plus running agents, keyed by transcript path."""
    sql = "(instance = '{0}' OR instance LIKE '{1}\\_%' ESCAPE '\\')".format(
        name.replace("'", "''"), name.replace("'", "''").replace("_", "\\_"))
    sessions = {}
    for e in events("--type", "life", "--action", "stopped", "--after", iso(start),
                    "--sql", sql, "--full"):
        s = e["data"].get("snapshot") or {}
        if s.get("transcript_path") and (s.get("created_at") or 0) < end.timestamp():
            sessions[s["transcript_path"]] = {
                "name": e["instance"], "tool": s.get("tool"), "path": s["transcript_path"],
                "directory": s.get("directory"), "agent_name": s.get("agent_name"),
                "state": "stopped " + parse_ts(e["ts"]).astimezone().strftime("%H:%M")}
    if datetime.now(timezone.utc) >= start:
        for a in json.loads(hcom("list", "--json")):
            if ((a["name"] == name or a["name"].startswith(name + "_")) and a.get("transcript_path")
                    and (a.get("created_at") or 0) < end.timestamp()):
                sessions[a["transcript_path"]] = {
                    "name": a["name"], "tool": a.get("tool"), "path": a["transcript_path"],
                    "directory": a.get("directory"),
                    "agent_name": (a.get("launch_context") or {}).get("agent_name"),
                    "state": "running"}
    return sessions, sql


def in_window(ex, start, end):
    ts = ex.get("timestamp")
    return not ts or start <= parse_ts(ts.replace("Z", "+00:00")) < end  # no ts: keep


def load_session(s, start, end):
    """Adds the session's exchanges in the window, token sums and rendered transcript to `s`."""
    exs = [ex for ex in json.loads(hcom("transcript", "--path", s["path"], "1-", "--detailed",
                                        "--json") or "[]") if in_window(ex, start, end)]
    s["exchanges"] = exs
    if not exs:
        return
    positions = [ex["position"] for ex in exs]
    s["range"] = f"{min(positions)}-{max(positions)}"
    s["text"] = hcom("transcript", "--path", s["path"], s["range"], "--detailed", "--full")
    tokens = {}
    for ex in exs:
        for k, v in (ex.get("usage") or {}).items():
            if isinstance(v, (int, float)):
                tokens[k] = tokens.get(k, 0) + v
    s["tokens"] = tokens or {"estimated": len(s["text"]) // 4}


def token_table(sessions):
    keys = sorted({k for s in sessions for k in s["tokens"]})
    total = {k: sum(s["tokens"].get(k, 0) for s in sessions) for k in keys}
    rows = ["| session | cli | state | exchanges | " + " | ".join(keys) + " |",
            "|" + " --- |" * (4 + len(keys))]
    for s in sessions + [{"name": "**total**", "tool": "", "state": "", "tokens": total,
                          "exchanges": [x for s in sessions for x in s["exchanges"]]}]:
        rows.append(f"| {s['name']} | {s['tool'] or ''} | {s['state']} | {len(s['exchanges'])} | "
                    + " | ".join(f"{s['tokens'].get(k, 0):,}" for k in keys) + " |")
    note = ("\n`estimated` = rendered transcript characters / 4, for CLIs without usage records."
            if "estimated" in keys else "")
    return "\n".join(rows) + note


def one_line(text, limit=160):
    text = " ".join((text or "").split())
    return text if len(text) <= limit else text[:limit - 1] + "…"


def timeline(sessions, sql, start, end):
    """Prompts, hcom messages, lifecycle, blocked approvals and gaps over GAP, by local time."""
    items = []  # (time, line, state for gap reports or None)
    for s in sessions:
        for ex in s["exchanges"]:
            if ex.get("timestamp") and ex.get("user"):
                items.append((parse_ts(ex["timestamp"].replace("Z", "+00:00")),
                              f"{s['name']} #{ex['position']} prompt: {one_line(ex['user'])}", None))
    for e in events("--type", "life", "--type", "status", "--after", iso(start),
                    "--before", iso(end), "--sql", sql):
        d, t = e["data"], parse_ts(e["ts"])
        if e["type"] == "life":
            items.append((t, f"{e['instance']} {d.get('action')} (by {d.get('by')})", None))
        elif d.get("status") == "blocked":
            items.append((t, f"{e['instance']} BLOCKED {d.get('context') or ''} "
                             f"{one_line(d.get('detail') or d.get('new_detail'))}",
                          f"{e['instance']} blocked"))
        else:
            items.append((t, None, f"{e['instance']} {d.get('status')} {d.get('context') or ''}"
                          .strip()))
    names = sorted({s["name"] for s in sessions})
    for e in events("--type", "message", "--after", iso(start), "--before", iso(end), "--full",
                    *[x for n in names for x in ("--participant", n)]):
        d = e["data"]
        items.append((parse_ts(e["ts"]), f"msg {d.get('from')} -> {','.join(d.get('delivered_to') or [])}"
                      f" [{d.get('intent') or '-'}] #{e['id']}: {one_line(d.get('text'))}", None))
    items.sort(key=lambda i: i[0])
    lines, prev, state = [], None, ""
    for t, line, st in items:
        if prev and (t - prev).total_seconds() > GAP:
            lines.append(f"- {t.astimezone():%H:%M:%S} GAP {int((t - prev).total_seconds() // 60)} min"
                         f" (state before: {state or 'unknown'})")
        prev, state = t, st or state
        if line:
            lines.append(f"- {t.astimezone():%H:%M:%S} {line}")
    return "\n".join(lines) or "(no events)"


def rule_files(rules, bundle_text):
    """(path, what) of every rule file named in the prompt lines, plus the bundle MEMORY files."""
    files = [(m.group(1), m.group(2)) for m in re.finditer(r"^- (/[^:\n]+): (.*)$", rules, re.M)]
    bundle = re.search(r"^bundle:[ \t]+(.+?)[ \t]*$", bundle_text, re.M)
    if bundle:
        files += [(p, "bundle MEMORY") for p in
                  sorted(glob.glob(os.path.join(bundle.group(1), "MEMORY", "**", "*.md"),
                                   recursive=True))]
    return files


def read_text(path):
    try:
        with open(path, errors="replace") as f:
            return f.read()
    except OSError as e:
        return f"(unreadable: {e})"


def day_review(args, stem):
    """Writes the day dump; returns (prompt, dump path, session count)."""
    start = datetime.strptime(args.day, "%Y-%m-%d").astimezone()
    end = (datetime.strptime(args.day, "%Y-%m-%d") + timedelta(days=1)).astimezone()  # DST-safe
    found, sql = day_sessions(args.agent, start, end)
    for s in found.values():
        load_session(s, start, end)
    sessions = sorted((s for s in found.values() if s["exchanges"]),
                      key=lambda s: s["exchanges"][0].get("timestamp") or "")
    if not sessions:
        sys.exit(f"review-session: no sessions of {args.agent} or its clones on {args.day}")
    # Rules come from NAME's own catalog entry (clones share it).
    base = next((s for s in reversed(sessions) if s["name"] == args.agent), sessions[-1])
    rules_file = stem + "-rules.txt"
    rules = rules_section({"tool": base["tool"], "directory": base["directory"],
                           "launch_context": {"agent_name": base["agent_name"]}}, rules_file)
    files = rule_files(rules, read_text(rules_file) if os.path.isfile(rules_file) else "")
    parts = [f"# Day dump: {args.agent} and clones, {args.day} "
             f"({start:%Z}, {start:%Y-%m-%d %H:%M} to {end:%Y-%m-%d %H:%M})",
             "## Token totals", token_table(sessions),
             "## Timeline (local time)", timeline(sessions, sql, start, end),
             "## Rules and memory", rules]
    parts += [f"### {path} ({what})\n\n{read_text(path)}" for path, what in files]
    parts.append("## Transcripts")
    parts += [f"### {s['name']} ({s['tool']}, {s['state']}) {s['path']} exchanges {s['range']}\n\n"
              f"{s['text']}" for s in sessions]
    dump = stem + "-dump.md"
    write_private(dump, "\n\n".join(parts) + "\n")
    listing = "\n".join(f"- {s['name']} ({s['tool']}, {s['state']}): exchanges {s['range']}"
                        for s in sessions)
    prompt = DAY_PROMPT.format(name=args.agent, day=args.day, dump=os.path.basename(dump),
                               sessions=listing)
    return prompt, dump, len(sessions)


def main(argv):
    p = argparse.ArgumentParser(prog="hcom run review-session", description=__doc__,
                                formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("agent", nargs="?", help="agent name (default: the agent in the Herdr pane)")
    p.add_argument("--pane", default=os.environ.get("HERDR_PANE_ID"), help="Herdr pane id")
    p.add_argument("--coach", default="agent_coach", help="recipient (default: agent_coach)")
    p.add_argument("--model", help="model (default: haiku for claude, gemini-4-neon-medium for agy)")
    p.add_argument("--day", help="review every session of NAME and NAME_* clones on this local "
                                 "day (YYYY-MM-DD)")
    p.add_argument("--cli", choices=sorted(DEFAULT_MODEL), default="claude",
                   help="analysis CLI (default: claude; agy needs --day)")
    p.add_argument("--name", help=argparse.SUPPRESS)  # caller identity passed by hcom run; unused
    args = p.parse_args(argv)
    if not args.agent and not args.pane:
        p.error("give NAME or --pane (no HERDR_PANE_ID)")
    if args.day and not args.agent:
        p.error("--day needs NAME")
    if args.cli == "agy" and not args.day:
        p.error("--cli agy needs --day (the dump is the only input agy reads)")
    model = args.model or DEFAULT_MODEL[args.cli]

    a = None if args.day else find_agent(args.agent, args.pane)
    name = args.agent if args.day else a["name"]
    outdir = os.path.join(os.environ.get("HCOM_DIR") or os.path.expanduser("~/.hcom"),
                          "tmp", "session-reviews")
    os.makedirs(outdir, exist_ok=True)
    stem = os.path.join(outdir, f"{name}-{'day-' + args.day + '-' if args.day else ''}"
                                f"{time.strftime('%Y%m%d-%H%M%S')}-{os.getpid()}")
    report = stem + ".md"
    if args.day:
        prompt, dump, count = day_review(args, stem)
    else:
        prompt =REVIEW_PROMPT.format(name=name, rules=rules_section(a, stem + "-rules.txt"),
                                      **{k: a.get(k) or "" for k in
                                         ("tool", "session_id", "transcript_path", "directory")})

    if args.cli == "agy":
        cmd = ["agy", "--model", model, "-p", prompt]
    else:
        cmd = ["claude", "-p", "--model", model, prompt]
    timeout = DAY_TIMEOUT if args.day else TIMEOUT
    print(f"review-session: reviewing {name} with {args.cli} {model}...", file=sys.stderr)
    try:
        run = subprocess.run(cmd, env=hcom_env(), cwd=outdir, capture_output=True, text=True,
                             timeout=timeout)
    except subprocess.TimeoutExpired:
        sys.exit(f"review-session: {args.cli} timed out after {timeout}s")
    if run.returncode or not run.stdout.strip():
        sys.exit(f"review-session: {args.cli} failed ({run.returncode}): {run.stderr.strip()}")
    text = STATUS_BLOCK.sub("", run.stdout).rstrip() + "\n"  # trailing turn-end block, not report
    write_private(report, text)

    if args.day:
        message = DAY_COACH_MESSAGE.format(name=name, day=args.day, count=count, model=model,
                                           report=report, dump=dump,
                                           summary=findings_summary(text))
    else:
        message = COACH_MESSAGE.format(name=name, model=model, report=report,
                                       summary=findings_summary(text))
    out = subprocess.run(["hcom", "send", f"@{args.coach}", "--from", "session-review",
                          "--intent", "inform"], input=message, env=hcom_env(),
                         capture_output=True, text=True)
    if out.returncode:
        sys.exit(f"review-session: report saved to {report}, send failed: {out.stderr.strip()}")
    print(report)


if __name__ == "__main__":
    main(sys.argv[1:])

#!/usr/bin/env python3
"""Tests for review_session.py: `python3 -I contrib/review-session/test_review_session.py`.

Fake `hcom` and `claude` executables on PATH record their arguments, environment and stdin;
nothing real is called.
"""

import json
import os
import subprocess
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
SCRIPT = os.path.join(HERE, "review_session.py")

AGENTS = [
    {"name": "luna", "tool": "claude", "session_id": "s-1", "transcript_path": "/t/luna.jsonl",
     "directory": "/p/luna", "launch_context": {"pane_id": "w1:pA", "terminal_preset_effective": "herdr"}},
    {"name": "nova", "tool": "codex", "session_id": "s-2", "transcript_path": "/t/nova.jsonl",
     "directory": "/p/nova", "launch_context": {"pane_id": "w1:pB", "terminal_preset_effective": "herdr"}},
]

CATALOG_KEYS = {"HCOM_AGENTS_FILE", "HCOM_AGENT_CATALOGS"}

FAKE = """#!/usr/bin/env python3
import json, os, sys
log = os.path.join(os.environ["FAKE_LOG"], os.path.basename(sys.argv[0]) + ".jsonl")
stdin = "" if sys.stdin.isatty() else sys.stdin.read()
with open(log, "a") as f:
    f.write(json.dumps({"argv": sys.argv[1:], "env": dict(os.environ), "stdin": stdin,
                        "cwd": os.getcwd()}) + "\\n")
if os.path.basename(sys.argv[0]) == "hcom" and sys.argv[1:3] == ["list", "--json"]:
    print(os.environ["FAKE_AGENTS"])
elif os.path.basename(sys.argv[0]) == "hcom" and sys.argv[1:3] == ["agent", "show"]:
    if os.environ.get("FAKE_SHOW_FAIL"):
        sys.exit("Error: " + os.environ["FAKE_SHOW_FAIL"])
    print(os.environ.get("FAKE_SHOW", ""))
elif os.path.basename(sys.argv[0]) == "hcom" and sys.argv[1] == "events":
    kind = "stopped" if "stopped" in sys.argv else "message" if "message" in sys.argv else "status"
    print(os.environ.get("FAKE_EVENTS_" + kind.upper(), ""))
elif os.path.basename(sys.argv[0]) == "hcom" and sys.argv[1] == "transcript":
    path = sys.argv[sys.argv.index("--path") + 1]
    if "--json" in sys.argv:
        print(json.dumps(json.loads(os.environ["FAKE_TRANSCRIPTS"])[path]))
    else:
        print("RENDERED " + path + " " + sys.argv[sys.argv.index("--path") + 2])
elif os.path.basename(sys.argv[0]) in ("claude", "agy"):
    print(os.environ.get("FAKE_CLAUDE_OUT", "# Session review: report body\\n\\nSTATUS: DONE\\nDone: review"))
"""

DAY = "2026-11-19"


def ts(hm):
    """UTC timestamp of DAY at local HH:MM, as hcom writes it."""
    from datetime import datetime, timezone
    return datetime.strptime(f"{DAY} {hm}", "%Y-%m-%d %H:%M").astimezone().astimezone(
        timezone.utc).strftime("%Y-%m-%dT%H:%M:%S")


def day_env():
    """cca_dev stopped at 12:00, its clone cca_dev_2 running; the stopped session also has an
    exchange the day before."""
    stopped = {"type": "life", "id": 1, "instance": "cca_dev", "ts": ts("12:00"),
               "data": {"action": "stopped", "snapshot": {
                   "transcript_path": "/t/a.jsonl", "tool": "claude", "directory": "/p",
                   "agent_name": "cca_dev", "created_at": 0}}}
    running = [{"name": "cca_dev_2", "tool": "codex", "transcript_path": "/t/b.jsonl",
                "directory": "/p", "created_at": 0, "launch_context": {"agent_name": "cca_dev"}},
               {"name": "other", "tool": "claude", "transcript_path": "/t/o.jsonl",
                "created_at": 0}]
    transcripts = {
        "/t/a.jsonl": [
            {"position": 1, "user": "old", "timestamp": "2026-11-17T10:00:00Z",
             "usage": {"input": 1000, "output": 1000}},
            {"position": 2, "user": "fix the bug", "timestamp": ts("09:00") + "Z",
             "usage": {"input": 10, "output": 20, "cache_read": 300}},
            {"position": 3, "user": "and test it", "timestamp": ts("09:30") + "Z",
             "usage": {"input": 1, "output": 2, "cache_read": 3}}],
        "/t/b.jsonl": [{"position": 1, "user": "clone work", "timestamp": ts("13:00") + "Z",
                        "usage": None}]}
    status = [{"type": "life", "id": 2, "instance": "cca_dev", "ts": ts("08:59"),
               "data": {"action": "created", "by": "cca_manager"}},
              {"type": "status", "id": 3, "instance": "cca_dev", "ts": ts("09:05"),
               "data": {"status": "listening", "context": "tool:Bash"}},
              {"type": "status", "id": 4, "instance": "cca_dev", "ts": ts("09:29"),
               "data": {"status": "blocked", "context": "approval", "detail": "git push"}}]
    msgs = [{"type": "message", "id": 9, "instance": "cca_dev", "ts": ts("09:10"),
             "data": {"from": "cca_dev", "delivered_to": ["agent_review"], "intent": "request",
                      "text": "please review"}}]
    lines = lambda rows: "\n".join(json.dumps(r) for r in rows)
    return {"FAKE_AGENTS": json.dumps(running), "FAKE_TRANSCRIPTS": json.dumps(transcripts),
            "FAKE_EVENTS_STOPPED": lines([stopped]), "FAKE_EVENTS_STATUS": lines(status),
            "FAKE_EVENTS_MESSAGE": lines(msgs),
            "FAKE_CLAUDE_OUT": "# Day review\n## Problems\n1. **[high] Waited on approval**\n"}


class ReviewSession(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        d = self.tmp.name
        self.bin = os.path.join(d, "bin")
        self.log = os.path.join(d, "log")
        self.hcom_dir = os.path.join(d, "hcomdir")
        os.makedirs(self.bin)
        os.makedirs(self.log)
        for tool in ("hcom", "claude", "agy"):
            path = os.path.join(self.bin, tool)
            with open(path, "w") as f:
                f.write(FAKE)
            os.chmod(path, 0o755)

    def tearDown(self):
        self.tmp.cleanup()

    def run_script(self, *args, extra_env=None):
        env = {"PATH": self.bin + os.pathsep + os.environ["PATH"], "HOME": self.tmp.name,
               "FAKE_LOG": self.log, "FAKE_AGENTS": json.dumps(AGENTS), "HCOM_DIR": self.hcom_dir,
               "HCOM_INSTANCE_NAME": "caller", "HCOM_PROCESS_ID": "pid-caller",
               "HCOM_LAUNCHED": "1", **(extra_env or {})}
        return subprocess.run([sys.executable, "-I", SCRIPT, *args], env=env,
                              stdin=subprocess.DEVNULL, capture_output=True, text=True)

    def calls(self, tool):
        path = os.path.join(self.log, tool + ".jsonl")
        with open(path) as f:
            return [json.loads(line) for line in f]

    def test_reviews_named_agent_and_sends_prompt_plus_report_to_default_coach(self):
        r = self.run_script("luna", "--name", "caller")
        self.assertEqual(r.returncode, 0, r.stderr)
        report = r.stdout.strip()
        self.assertTrue(report.startswith(os.path.join(self.hcom_dir, "tmp", "session-reviews", "luna-")))
        with open(report) as f:
            self.assertEqual(f.read(), "# Session review: report body\n")  # STATUS block cut
        self.assertEqual(os.stat(report).st_mode & 0o777, 0o600)

        (claude,) = self.calls("claude")
        self.assertEqual(claude["argv"][:3], ["-p", "--model", "haiku"])
        prompt = claude["argv"][3]
        for value in ("luna", "claude", "s-1", "/t/luna.jsonl", "/p/luna"):
            self.assertIn(value, prompt)

        send = self.calls("hcom")[-1]
        self.assertEqual(send["argv"][:2], ["send", "@agent_coach"])
        self.assertTrue(send["stdin"].startswith("Independent session review of agent luna"))
        self.assertIn("no findings.", send["stdin"])
        self.assertTrue(send["stdin"].rstrip().endswith("Report: " + report))
        self.assertNotIn("report body", send["stdin"])  # path only, never the report

    def test_message_carries_count_and_top_finding_only(self):
        body = ("# Session review: luna\n## Conclusion\nBad.\n## Problems\n"
                "1. **[high] Premature DONE** details\n   evidence 2. not an item\n"
                "2. **[low] Duplicate text**\n## Minor\n1. sed -i allowed\n## Uncertain\nnone")
        r = self.run_script("luna", extra_env={"FAKE_CLAUDE_OUT": body})
        self.assertEqual(r.returncode, 0, r.stderr)
        msg = self.calls("hcom")[-1]["stdin"]
        self.assertIn(": 2 findings; top: [high] Premature DONE details\n", msg)
        self.assertNotIn("Duplicate text", msg)
        self.assertNotIn("Bad.", msg)

    def catalog_agent(self, tool="claude"):
        d = os.path.join(self.tmp.name, "proj")
        bundle = os.path.join(self.tmp.name, "my bundle")  # space: the path is not cut
        for path in (d, bundle, os.path.join(self.tmp.name, ".claude"),
                     os.path.join(self.tmp.name, ".codex")):
            os.makedirs(path, exist_ok=True)
        for path in (os.path.join(d, "AGENTS.md"), os.path.join(bundle, "NOTES.md"),
                     os.path.join(self.tmp.name, ".claude", "CLAUDE.md"),
                     os.path.join(self.tmp.name, ".codex", "AGENTS.md")):
            open(path, "w").close()
        agent = {"name": "mira_x", "tool": tool, "directory": d,
                 "launch_context": {"agent_name": "mira"}}
        return agent, d, bundle

    def test_prompt_names_effective_rules_of_a_catalog_agent(self):
        agent, d, bundle = self.catalog_agent()
        show = f"name: mira\nbundle:    {bundle}\ninstructions: {bundle}/SOUL.md\n"
        r = self.run_script("mira_x", extra_env={
            "FAKE_AGENTS": json.dumps([agent]), "FAKE_SHOW": show,
            "HCOM_AGENTS_FILE": "/c/agents.json", "HCOM_AGENT_CATALOGS": "/c/x.json"})
        self.assertEqual(r.returncode, 0, r.stderr)
        (call,) = [c for c in self.calls("hcom") if c["argv"][:2] == ["agent", "show"]]
        self.assertEqual(call["argv"], ["agent", "show", "mira"])
        self.assertEqual(call["cwd"], d)
        self.assertEqual({k for k in call["env"] if k.startswith("HCOM_")},
                         {"HCOM_DIR", "HCOM_AGENTS_FILE", "HCOM_AGENT_CATALOGS"})
        rules = r.stdout.strip()[:-len(".md")] + "-rules.txt"
        with open(rules) as f:
            self.assertEqual(f.read(), show + "\n")
        self.assertEqual(os.stat(rules).st_mode & 0o777, 0o600)
        prompt = self.calls("claude")[0]["argv"][3]
        for path in (rules, os.path.join(bundle, "NOTES.md"), os.path.join(d, "AGENTS.md"),
                     os.path.join(self.tmp.name, ".claude", "CLAUDE.md")):
            self.assertIn(f"- {path}:", prompt)
        self.assertNotIn(".codex", prompt)
        claude_env = self.calls("claude")[0]["env"]
        self.assertFalse(CATALOG_KEYS & claude_env.keys())

    def test_codex_agent_gets_codex_global_instructions(self):
        agent, _, _ = self.catalog_agent("codex")
        r = self.run_script("mira_x", extra_env={"FAKE_AGENTS": json.dumps([agent])})
        self.assertEqual(r.returncode, 0, r.stderr)
        prompt = self.calls("claude")[0]["argv"][3]
        self.assertIn(os.path.join(self.tmp.name, ".codex", "AGENTS.md") + ": codex", prompt)
        self.assertNotIn("CLAUDE.md", prompt)

    def test_failed_catalog_lookup_is_not_no_catalog_entry(self):
        agent, _, _ = self.catalog_agent()
        r = self.run_script("mira_x", extra_env={"FAKE_AGENTS": json.dumps([agent]),
                                                 "FAKE_SHOW_FAIL": "catalog broken"})
        self.assertEqual(r.returncode, 0, r.stderr)
        prompt = self.calls("claude")[0]["argv"][3]
        self.assertIn("Catalog rules could not be retrieved (Error: catalog broken)", prompt)
        self.assertIn("uncertain", prompt)
        self.assertNotIn("No catalog entry", prompt)
        self.assertFalse(os.path.exists(r.stdout.strip()[:-len(".md")] + "-rules.txt"))

    def test_missing_agent_directory_is_a_failed_lookup(self):
        agent, _, _ = self.catalog_agent()
        agent["directory"] = os.path.join(self.tmp.name, "gone")
        r = self.run_script("mira_x", extra_env={"FAKE_AGENTS": json.dumps([agent])})
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertIn("Catalog rules could not be retrieved", self.calls("claude")[0]["argv"][3])

    def test_agent_without_catalog_name_skips_lookup(self):
        r = self.run_script("luna")
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertFalse([c for c in self.calls("hcom") if c["argv"][:2] == ["agent", "show"]])
        self.assertIn("No catalog entry (ad hoc agent)", self.calls("claude")[0]["argv"][3])

    def test_children_do_not_inherit_hcom_identity_or_caller_cli_markers(self):
        markers = {"CLAUDECODE": "1", "CLAUDE_ENV_FILE": "/e", "CLAUDE_CODE_SESSION_ID": "c",
                   "CODEX_THREAD_ID": "t", "CODEX_SESSION_ID": "s"}
        r = self.run_script("luna", extra_env=markers)
        self.assertEqual(r.returncode, 0, r.stderr)
        for call in self.calls("claude") + self.calls("hcom"):
            hcom_keys = {k for k in call["env"] if k.startswith("HCOM_")}
            self.assertEqual(hcom_keys, {"HCOM_DIR"})
            self.assertFalse(markers.keys() & call["env"].keys())

    def test_status_block_cut_only_at_the_end(self):
        body = "# Session review: x\n## Problems\nquoted:\nSTATUS: DONE\nDone: y\n## Uncertain\nnone"
        r = self.run_script("luna", extra_env={"FAKE_CLAUDE_OUT": body})
        self.assertEqual(r.returncode, 0, r.stderr)
        with open(r.stdout.strip()) as f:
            self.assertEqual(f.read(), body + "\n")

    def test_coach_and_model_from_arguments(self):
        r = self.run_script("luna", "--coach", "hcom_coach", "--model", "sonnet")
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(self.calls("claude")[0]["argv"][2], "sonnet")
        self.assertEqual(self.calls("hcom")[-1]["argv"][1], "@hcom_coach")

    def test_agent_from_herdr_pane_env(self):
        r = self.run_script(extra_env={"HERDR_PANE_ID": "w1:pB"})
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertIn("nova", self.calls("claude")[0]["argv"][3])

    def test_pane_argument_overrides_env_and_name_overrides_pane(self):
        r = self.run_script("--pane", "w1:pA", extra_env={"HERDR_PANE_ID": "w1:pB"})
        self.assertEqual(r.returncode, 0, r.stderr)
        r = self.run_script("nova", "--pane", "w1:pA")
        self.assertEqual(r.returncode, 0, r.stderr)
        prompts = [c["argv"][3] for c in self.calls("claude")]
        self.assertIn("Agent: luna", prompts[0])
        self.assertIn("Agent: nova", prompts[1])

    def test_pane_must_match_exactly_one_herdr_agent(self):
        other = {"name": "kiro", "launch_context": {"pane_id": "w1:pA", "terminal_preset_effective": "kitty"}}
        twin = {"name": "dero", "launch_context": AGENTS[0]["launch_context"]}
        r = self.run_script("--pane", "w1:pA", extra_env={"FAKE_AGENTS": json.dumps([*AGENTS, other])})
        self.assertEqual(r.returncode, 0, r.stderr)  # non-herdr row with the same id is ignored
        r = self.run_script("--pane", "w1:pA", extra_env={"FAKE_AGENTS": json.dumps([*AGENTS, twin])})
        self.assertNotEqual(r.returncode, 0)
        self.assertIn("several agents in pane w1:pA: luna, dero", r.stderr)
        r = self.run_script("--pane", "w9:pZ")
        self.assertNotEqual(r.returncode, 0)
        self.assertIn("no running agent in pane w9:pZ", r.stderr)
        self.assertEqual(len(self.calls("claude")), 1)

    def test_unknown_agent_fails_without_calling_claude(self):
        r = self.run_script("zeta")
        self.assertNotEqual(r.returncode, 0)
        self.assertIn("no running agent zeta", r.stderr)
        self.assertFalse(os.path.exists(os.path.join(self.log, "claude.jsonl")))

    def test_day_review_on_agy_dumps_all_sessions_of_the_day(self):
        r = self.run_script("cca_dev", "--day", DAY, "--cli", "agy", extra_env=day_env())
        self.assertEqual(r.returncode, 0, r.stderr)
        report = r.stdout.strip()
        self.assertIn(f"cca_dev-day-{DAY}-", report)
        (agy,) = self.calls("agy")
        self.assertEqual(agy["argv"][:3], ["--model", "gemini-4-neon-medium", "-p"])
        outdir = os.path.join(self.hcom_dir, "tmp", "session-reviews")
        self.assertEqual(agy["cwd"], outdir)
        dump = report[:-len(".md")] + "-dump.md"
        self.assertIn(os.path.basename(dump), agy["argv"][3])
        self.assertIn("- cca_dev (claude, stopped 12:00): exchanges 2-3", agy["argv"][3])
        with open(dump) as f:
            text = f.read()
        self.assertEqual(os.stat(dump).st_mode & 0o777, 0o600)
        # Exchange 1 is from another day: not rendered, not counted.
        self.assertIn("RENDERED /t/a.jsonl 2-3", text)
        self.assertIn("RENDERED /t/b.jsonl 1-1", text)
        self.assertNotIn("/t/o.jsonl", text)
        self.assertIn("| cca_dev | claude | stopped 12:00 | 2 | 303 | 0 | 11 | 22 |", text)
        self.assertIn("| cca_dev_2 | codex | running | 1 | 0 | ", text)  # no usage: estimated
        self.assertIn("| **total** |  |  | 3 | 303 | ", text)
        self.assertIn("09:00:00 cca_dev #2 prompt: fix the bug", text)
        self.assertIn("09:10:00 msg cca_dev -> agent_review [request] #9: please review", text)
        self.assertIn("09:29:00 GAP 19 min (state before: cca_dev listening tool:Bash)", text)
        self.assertIn("09:29:00 cca_dev BLOCKED approval git push", text)
        self.assertIn("13:00:00 GAP 210 min (state before: cca_dev blocked)", text)
        participants = [c["argv"] for c in self.calls("hcom") if "message" in c["argv"]][0]
        self.assertIn("cca_dev_2", participants)
        msg = self.calls("hcom")[-1]["stdin"]
        self.assertIn(f"Day review of agent cca_dev and clones for {DAY} (2 sessions)", msg)
        self.assertIn("1 finding; top: [high] Waited on approval", msg)
        self.assertIn("Dump (tokens, timeline, transcripts): " + dump, msg)

    def test_day_review_inlines_rules_notes_and_memory(self):
        bundle = os.path.join(self.tmp.name, "bundle")
        os.makedirs(os.path.join(bundle, "MEMORY"))
        for name, body in (("NOTES.md", "note line"), ("MEMORY/topic.md", "memory line")):
            with open(os.path.join(bundle, name), "w") as f:
                f.write(body)
        os.makedirs(os.path.join(self.tmp.name, "p"))
        env = day_env()
        env["FAKE_EVENTS_STOPPED"] = env["FAKE_EVENTS_STOPPED"].replace(
            '"/p"', json.dumps(os.path.join(self.tmp.name, "p")))
        env["FAKE_SHOW"] = f"name: cca_dev\nbundle: {bundle}\nSOUL rule line"
        r = self.run_script("cca_dev", "--day", DAY, extra_env=env)
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(self.calls("claude")[0]["argv"][:3], ["-p", "--model", "haiku"])
        with open(r.stdout.strip()[:-len(".md")] + "-dump.md") as f:
            text = f.read()
        for line in ("SOUL rule line", "note line", "memory line"):
            self.assertIn(line, text)

    def test_agy_needs_day_and_day_needs_sessions(self):
        r = self.run_script("luna", "--cli", "agy")
        self.assertNotEqual(r.returncode, 0)
        self.assertIn("--cli agy needs --day", r.stderr)
        r = self.run_script("zeta", "--day", DAY, extra_env={**day_env(), "FAKE_EVENTS_STOPPED": "",
                                                            "FAKE_AGENTS": "[]"})
        self.assertNotEqual(r.returncode, 0)
        self.assertIn(f"no sessions of zeta or its clones on {DAY}", r.stderr)
        self.assertFalse(os.path.exists(os.path.join(self.log, "agy.jsonl")))
        self.assertFalse(os.path.exists(os.path.join(self.log, "claude.jsonl")))


if __name__ == "__main__":
    unittest.main()

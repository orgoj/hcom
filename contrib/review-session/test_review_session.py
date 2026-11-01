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

FAKE = """#!/usr/bin/env python3
import json, os, sys
log = os.path.join(os.environ["FAKE_LOG"], os.path.basename(sys.argv[0]) + ".jsonl")
stdin = "" if sys.stdin.isatty() else sys.stdin.read()
with open(log, "a") as f:
    f.write(json.dumps({"argv": sys.argv[1:], "env": dict(os.environ), "stdin": stdin}) + "\\n")
if os.path.basename(sys.argv[0]) == "hcom" and sys.argv[1:3] == ["list", "--json"]:
    print(os.environ["FAKE_AGENTS"])
elif os.path.basename(sys.argv[0]) == "claude":
    print(os.environ.get("FAKE_CLAUDE_OUT", "# Session review: report body\\n\\nSTATUS: DONE\\nDone: review"))
"""


class ReviewSession(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        d = self.tmp.name
        self.bin = os.path.join(d, "bin")
        self.log = os.path.join(d, "log")
        self.hcom_dir = os.path.join(d, "hcomdir")
        os.makedirs(self.bin)
        os.makedirs(self.log)
        for tool in ("hcom", "claude"):
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
        self.assertIn(report, send["stdin"])
        self.assertTrue(send["stdin"].rstrip().endswith("# Session review: report body"))

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


if __name__ == "__main__":
    unittest.main()

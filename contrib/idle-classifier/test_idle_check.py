#!/usr/bin/env python3
"""Fixture tests for idle_check.py and final_inputs.final_input: python3 -I test_idle_check.py"""
import io
import json
import pathlib
import sys
import tempfile
import unittest
from contextlib import redirect_stderr, redirect_stdout

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import idle_check  # noqa: E402
from final_inputs import MAX_CHARS, final_input  # noqa: E402

SCORER = ["py", "attention.py", "score", "head"]
SECRET = "the agent's private final message"


def agent(name, status="listening", age=5, tool="claude"):
    return {"name": name, "status": status, "status_age_seconds": age, "tool": tool}


def exchange(text, position=1, timestamp="2026-10-25T00:00:00Z"):
    return {"position": position, "timestamp": timestamp, "final_text": text, "action": text, "tools": []}


def ok(value):
    return 0, json.dumps(value), ""


def score_row(i, cls="done", needs_user=0.1):
    rest = (1 - needs_user) / 2
    return {"id": str(i), "class": cls, "must_act": cls == "needs_user", "p_must_act": needs_user,
            "p": {"needs_user": needs_user, "waiting": rest, "done": rest}, "latency_ms": 50}


class Fake:
    """Runner answering hcom list/transcript and the scorer from queued responses."""

    def __init__(self, lists, transcripts=None, score=None):
        self.lists, self.transcripts, self.score_out = list(lists), transcripts or {}, score
        self.score_stdin = None

    def __call__(self, argv, stdin=None):
        if argv[1:3] == ["list", "--json"]:
            return self.lists.pop(0)
        if argv[1] == "transcript":
            queue = self.transcripts[argv[2]]
            return queue.pop(0) if len(queue) > 1 else queue[0]
        assert argv == SCORER
        self.score_stdin = stdin
        return self.score_out(stdin) if callable(self.score_out) else self.score_out


def scores_for(classes):
    """Scorer answering every input row with the given (class, p_needs_user) in order."""
    def answer(stdin):
        ids = [json.loads(line)["id"] for line in stdin.splitlines()]
        return 0, "".join(json.dumps(score_row(i, *classes[int(i)])) + "\n" for i in ids), ""
    return answer


def check(fake):
    return idle_check.check(fake, "hcom", SCORER)


class Check(unittest.TestCase):
    def test_no_listening_agents_skips_scorer(self):
        fake = Fake([ok([agent("a", "active")])], score=lambda s: self.fail("scorer called"))
        self.assertEqual(check(fake), [])

    def test_per_agent_reasons(self):
        listed = ok([agent("empty"), agent("failed"), agent("bad"), agent("blank"), agent("notlist")])
        fake = Fake([listed, listed], {
            "empty": [ok([])], "failed": [(1, SECRET, SECRET)], "bad": [(0, SECRET, "")],
            "blank": [ok([exchange("   ")])], "notlist": [ok({"x": 1})],
        }, score=lambda s: self.fail("scorer called"))
        reasons = {r["name"]: (r["class"], r["reason"]) for r in check(fake)}
        self.assertEqual(reasons, {"empty": (None, "no_exchange"), "failed": (None, "transcript_failed"),
                                   "bad": (None, "transcript_invalid"), "blank": (None, "empty_text"),
                                   "notlist": (None, "transcript_invalid")})

    def test_schema_sort_and_input(self):
        listed = ok([agent("b"), agent("a"), agent("c"), agent("d"), agent("e"), agent("gone", "active")])
        fake = Fake([listed, ok([agent("b", age=9), agent("a"), agent("c"), agent("d"), agent("e")])], {
            "a": [ok([exchange("one")])], "b": [ok([exchange("two")])], "c": [ok([exchange("three")])],
            "d": [ok([exchange("four")])], "e": [ok([])],
        }, score=scores_for([("needs_user", 0.5), ("done", 0.1), ("needs_user", 0.5), ("waiting", 0.9)]))
        rows = check(fake)
        self.assertEqual([(r["name"], r["class"]) for r in rows],
                         [("b", "needs_user"), ("c", "needs_user"), ("d", "waiting"), ("a", "done"), ("e", None)])
        self.assertEqual(set(rows[0]), {"name", "tool", "class", "p_needs_user", "p", "input_kind",
                                        "status_age_seconds", "reason"})
        self.assertEqual((rows[0]["p_needs_user"], rows[0]["input_kind"], rows[0]["status_age_seconds"],
                          rows[0]["reason"]), (0.5, "final_text", 9, None))
        self.assertEqual([json.loads(line) for line in fake.score_stdin.splitlines()],
                         [{"id": "0", "text": "two"}, {"id": "1", "text": "one"},
                          {"id": "2", "text": "three"}, {"id": "3", "text": "four"}])

    def test_stale(self):
        listed = ok([agent("left"), agent("gone"), agent("changed"), agent("failed"), agent("remote", age=0)])
        fake = Fake([listed, ok([agent("left", "active"), agent("changed"), agent("failed"),
                                 agent("remote", age=0)])], {
            "left": [ok([exchange("x")])], "gone": [ok([exchange("x")])],
            "changed": [ok([exchange("x")]), ok([exchange("y", position=2)])],
            "failed": [ok([exchange("x")]), (1, "", "")], "remote": [ok([exchange("x")])],
        }, score=scores_for([("done", 0.1)] * 5))
        got = {r["name"]: (r["class"], r["reason"]) for r in check(fake)}
        self.assertEqual(got, {"left": (None, "stale"), "gone": (None, "stale"), "changed": (None, "stale"),
                               "failed": (None, "stale"), "remote": ("done", None)})

    def test_bad_score_output_fails_without_text(self):
        good = json.dumps(score_row(0)) + "\n"
        cases = {
            "nonzero": (1, SECRET, SECRET),
            "not json": (0, SECRET + "\n", ""),
            "duplicate": (0, good + good, ""),
            "missing": (0, "", ""),
            "extra": (0, good + json.dumps(score_row(1)) + "\n", ""),
            "bad class": (0, json.dumps(score_row(0, "maybe")) + "\n", ""),
            "p > 1": (0, json.dumps(score_row(0, "done", 1.5)) + "\n", ""),
            "p NaN": (0, json.dumps(score_row(0, "done", float("nan"))) + "\n", ""),
            "p keys": (0, json.dumps({**score_row(0), "p": {"needs_user": 0.1}}) + "\n", ""),
        }
        for name, out in cases.items():
            with self.subTest(name):
                fake = Fake([ok([agent("a")])], {"a": [ok([exchange(SECRET)])]}, score=out)
                with self.assertRaises(idle_check.Failure) as raised:
                    check(fake)
                self.assertNotIn(SECRET, str(raised.exception))

    def test_bad_list_fails(self):
        for out in [(1, SECRET, SECRET), (0, SECRET, ""), ok({"a": 1}), ok([{"name": "a"}])]:
            with self.subTest(out), self.assertRaises(idle_check.Failure) as raised:
                check(Fake([out]))
            self.assertNotIn(SECRET, str(raised.exception))

    def test_main_text_and_json(self):
        with tempfile.TemporaryDirectory() as tmp:
            for f in ("py", "attention.py", "head.json"):
                pathlib.Path(tmp, f).touch()
            flags = ["--python", f"{tmp}/py", "--attention", f"{tmp}/attention.py", "--head", tmp]
            scorer = [f"{tmp}/py", f"{tmp}/attention.py", "score", tmp]

            def runner(argv, stdin=None):
                if argv[0] == "hcom":
                    return fake(argv, stdin)
                self.assertEqual(argv, scorer)
                return fake([*SCORER], stdin)

            for as_json in (False, True):
                fake = Fake([ok([agent("a"), agent("b")])] * 2,
                            {"a": [ok([exchange(SECRET)])], "b": [ok([])]}, score=scores_for([("needs_user", 0.25)]))
                out, err = io.StringIO(), io.StringIO()
                with redirect_stdout(out), redirect_stderr(err):
                    code = idle_check.main(flags + ["--json"] * as_json, runner)
                self.assertEqual((code, err.getvalue()), (0, ""))
                self.assertNotIn(SECRET, out.getvalue())
                if as_json:
                    self.assertEqual([r["name"] for r in json.loads(out.getvalue())], ["a", "b"])
                else:
                    self.assertEqual(out.getvalue().splitlines(), [
                        "NAME  CLASS            P_NEEDS_USER", "a     needs_user       0.250",
                        "b     - (no_exchange)  -"])

    def test_main_failure_is_a_category(self):
        err = io.StringIO()
        with redirect_stderr(err):
            code = idle_check.main(["--python", "/nonexistent/py"], Fake([]))
        self.assertEqual((code, err.getvalue()), (1, "idle_check: --python: /nonexistent/py not found\n"))


class FinalInput(unittest.TestCase):
    def test_kinds(self):
        bash = {"name": "Bash", "command": "cd x && hcom send @a --name me -- 'hi'"}
        codex = {"name": "exec", "command": 'tools.exec_command({"cmd": "ls"}); '
                                            'tools.exec_command({"cmd": "hcom send @a -- \\"ok\\""})'}
        cases = [
            ({"final_text": " done ", "action": "ignored", "tools": [bash]}, ("final_text", "done")),
            ({"action": " reply ", "tools": []}, ("tool_free", "reply")),
            ({"action": "text", "tools": [bash]}, ("hcom_send", "text\n\nhcom send @a --name me -- 'hi'")),
            ({"action": "", "tools": [codex]}, ("hcom_send", 'hcom send @a -- "ok"')),
            ({"action": "text", "tools": [{"name": "Bash", "command": "ls"}]}, ("tool_ended_other", "text")),
            ({"action": "text", "tools": [{"name": "Read", "command": "hcom send @a -- x"}]},
             ("tool_ended_other", "text")),
        ]
        for ex, want in cases:
            with self.subTest(want):
                self.assertEqual(final_input(ex), want)

    def test_tail(self):
        kind, text = final_input({"final_text": "a" + "b" * MAX_CHARS})
        self.assertEqual((kind, text), ("final_text", "b" * MAX_CHARS))


if __name__ == "__main__":
    unittest.main()

#!/usr/bin/env python3
"""Fixture tests for idle_check.py and final_inputs.final_input: python3 -I test_idle_check.py"""
import hashlib
import io
import json
import os
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


def sha(text):
    return hashlib.sha1(text.encode()).hexdigest()


def lines(path):
    return path.read_text().splitlines() if path.exists() else []


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
                                        "status_age_seconds", "reason", "text_sha1", "corrected"})
        self.assertEqual((rows[0]["p_needs_user"], rows[0]["input_kind"], rows[0]["status_age_seconds"],
                          rows[0]["reason"], rows[0]["text_sha1"], rows[0]["corrected"]),
                         (0.5, "final_text", 9, None, sha("two"), False))
        self.assertEqual((rows[-1]["text_sha1"], rows[-1]["corrected"]), (None, False))
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

    def test_names_filter(self):
        listed = ok([agent("a"), agent("b"), agent("c", "active")])
        fake = Fake([listed, listed], {"a": [ok([exchange("x")])], "b": [ok([exchange("y")])]},
                    score=scores_for([("waiting", 0.2)]))
        rows = idle_check.check(fake, "hcom", SCORER, {"b", "c", "unknown"})
        self.assertEqual([(r["name"], r["class"]) for r in rows], [("b", "waiting")])
        self.assertEqual([json.loads(line)["text"] for line in fake.score_stdin.splitlines()], ["y"])

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
            flags = ["--python", f"{tmp}/py", "--attention", f"{tmp}/attention.py", "--head", tmp,
                     "--live", f"{tmp}/live"]
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


class Live(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.dir = pathlib.Path(tmp.name)
        self.store = idle_check.Store(self.dir / "live")
        self.log = self.dir / "live" / "scores.jsonl"

    def run_check(self, texts, score, model="m1"):
        """check() over agents a, b, ... whose last exchanges are texts."""
        names = [chr(ord("a") + i) for i in range(len(texts))]
        listed = ok([agent(n) for n in names])
        fake = Fake([listed, listed], {n: [ok([exchange(t)])] for n, t in zip(names, texts)}, score=score)
        return idle_check.check(fake, "hcom", SCORER, None, self.store, model, "/heads/h"), fake

    def test_cache_scores_each_text_once(self):
        rows, _ = self.run_check(["one"], scores_for([("waiting", 0.2)]))
        self.assertEqual([(r["class"], r["text_sha1"]) for r in rows], [("waiting", sha("one"))])
        rows, fake = self.run_check(["one"], lambda s: self.fail("scorer called"))
        self.assertEqual([r["class"] for r in rows], ["waiting"])
        rows, fake = self.run_check(["one", "two"], scores_for([("done", 0.1)]))
        self.assertEqual([json.loads(line)["text"] for line in fake.score_stdin.splitlines()], ["two"])
        logged = [json.loads(line) for line in lines(self.log)]
        self.assertEqual([(r["name"], r["text"], r["class"], r["model"], r["head"]) for r in logged],
                         [("a", "one", "waiting", "m1", "/heads/h"), ("b", "two", "done", "m1", "/heads/h")])
        self.assertEqual(set(logged[0]), {"ts", "name", "tool", "input_kind", "text_sha1", "text", "model", "head",
                                          "p", "class"})
        self.run_check(["one"], scores_for([("needs_user", 0.9)]), model="m2")  # another model scores again
        self.assertEqual(len(lines(self.log)), 3)

    def test_same_text_twice_in_a_batch_is_one_input(self):
        rows, fake = self.run_check(["same", "same"], scores_for([("needs_user", 0.7)]))
        self.assertEqual(len(fake.score_stdin.splitlines()), 1)
        self.assertEqual([r["class"] for r in rows], ["needs_user", "needs_user"])
        self.assertEqual(len(lines(self.log)), 1)

    def test_a_concurrent_writer_is_not_duplicated(self):
        other = scores_for([("done", 0.1)])

        def score(stdin):  # another idle_check logs the same text while this one scores
            idle_check.Store(self.dir / "live").add_scores("m1", [{
                **score_row(0, "done", 0.1), "text_sha1": sha("one"), "model": "m1", "text": "one"}])
            return other(stdin)

        self.run_check(["one"], score)
        self.assertEqual(len(lines(self.log)), 1)

    def test_invalid_cached_rows_are_skipped(self):
        self.log.parent.mkdir()
        good = {"text_sha1": sha("one"), "model": "m1", **score_row(0, "done", 0.1)}
        bad = [{**good, "class": "maybe"}, {**good, "p": {"needs_user": 2, "waiting": 0, "done": 0}},
               {**good, "text_sha1": "x"}, [1], "text"]
        self.log.write_text("".join(json.dumps(b) + "\n" for b in bad) + "{broken\n")
        rows, fake = self.run_check(["one"], scores_for([("waiting", 0.3)]))
        self.assertIsNotNone(fake.score_stdin)
        self.assertEqual(rows[0]["class"], "waiting")

    def test_torn_tail_does_not_eat_the_next_record(self):
        self.log.parent.mkdir()
        corrections = self.log.parent / "corrections.jsonl"
        self.log.write_text('{"text_sha1": "')
        corrections.write_text('{"ts": "x", "text_')
        self.run_check(["one"], scores_for([("waiting", 0.3)]))
        self.store.label(sha("one"), "done")
        self.assertEqual(list(self.store.scores("m1")), [sha("one")])
        self.assertEqual(self.store.corrections(), {sha("one"): "done"})

    def test_files_are_private(self):
        live = self.log.parent
        live.mkdir(mode=0o755)
        for f in ("scores.jsonl", "corrections.jsonl", ".lock"):
            (live / f).touch(mode=0o644)
        self.run_check(["one"], scores_for([("done", 0.1)]))
        self.store.label(sha("one"), "done")
        modes = {f.name: f.stat().st_mode & 0o777 for f in live.iterdir()}
        self.assertEqual((live.stat().st_mode & 0o777, modes),
                         (0o700, {"scores.jsonl": 0o600, "corrections.jsonl": 0o600, ".lock": 0o600}))

    def test_a_cache_only_run_tightens_existing_files(self):
        self.run_check(["one"], scores_for([("done", 0.1)]))
        self.store.label(sha("one"), "waiting")
        live = self.log.parent
        for f in live.iterdir():
            f.chmod(0o644)
        live.chmod(0o755)
        rows, _ = self.run_check(["one"], lambda s: self.fail("scorer called"))
        self.assertEqual(rows[0]["class"], "waiting")
        self.assertEqual((live.stat().st_mode & 0o777, {f.name: f.stat().st_mode & 0o777 for f in live.iterdir()}),
                         (0o700, {"scores.jsonl": 0o600, "corrections.jsonl": 0o600, ".lock": 0o600}))

    def test_last_correction_wins(self):
        self.store.label(sha("one"), "needs_user")
        self.store.label(sha("one"), "done")
        rows, _ = self.run_check(["one", "two"], scores_for([("needs_user", 0.9), ("waiting", 0.2)]))
        self.assertEqual([(r["name"], r["class"], r["corrected"], r["p_needs_user"]) for r in rows],
                         [("b", "waiting", False, 0.2), ("a", "done", True, 0.9)])

    def test_label_needs_no_model(self):
        live = str(self.dir / "live")
        missing = ["--python", "/nonexistent/py", "--attention", "/nonexistent/a.py", "--head", "/nonexistent"]
        err = io.StringIO()
        with redirect_stderr(err):
            self.assertEqual(idle_check.main(missing + ["--live", live, "--label", sha("x"), "waiting"], Fake([])), 0)
            for bad in (["XYZ", "waiting"], [sha("x"), "maybe"]):
                self.assertEqual(idle_check.main(["--live", live, "--label", *bad], Fake([])), 1)
        self.assertEqual(self.store.corrections(), {sha("x"): "waiting"})
        self.assertEqual(err.getvalue().count("idle_check: --label:"), 2)

    def test_model_id(self):
        attention = self.dir / "attention.py"
        attention.write_text("v1")
        heads = []
        for parent in ("x", "y"):
            head = self.dir / parent / "head"
            head.mkdir(parents=True)
            (head / "head.json").write_text("{}")
            (head / "head.safetensors").write_bytes(b"x")
            os.utime(head / "head.safetensors", ns=(1, 1))
            heads.append(head)
        same = idle_check.model_id(attention, heads[0])
        self.assertEqual(same, idle_check.model_id(attention, heads[1]))  # same content, other dir
        (heads[1] / "head.safetensors").write_bytes(b"y")  # other weights, same name, size and mtime
        os.utime(heads[1] / "head.safetensors", ns=(1, 1))
        self.assertNotEqual(same, idle_check.model_id(attention, heads[1]))
        (heads[0] / "head.json").write_text('{"v": 2}')
        self.assertNotEqual(same, idle_check.model_id(attention, heads[0]))
        second = idle_check.model_id(attention, heads[0])
        attention.write_text("v2")
        self.assertNotEqual(second, idle_check.model_id(attention, heads[0]))

    def test_model_id_of_large_files_is_their_identity(self):
        attention = self.dir / "attention.py"
        attention.write_text("v1")
        head = self.dir / "head"
        head.mkdir()
        (head / "head.json").write_text("{}")
        (head / "model.safetensors").write_bytes(b"12345")
        old = idle_check.HASH_LIMIT
        idle_check.HASH_LIMIT = 4
        self.addCleanup(setattr, idle_check, "HASH_LIMIT", old)
        first = idle_check.model_id(attention, head)
        self.assertEqual(first, idle_check.model_id(attention, head))
        (head / "model.safetensors").write_bytes(b"54321")  # rewritten in place: ctime/mtime move
        os.utime(head / "model.safetensors", ns=(1, 1))
        self.assertNotEqual(first, idle_check.model_id(attention, head))


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

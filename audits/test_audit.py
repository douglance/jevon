"""Tests for the audit tooling itself.

The tooling is Python in a Rust workspace, so no cargo gate covers it. Without
these, a change to `audit.py` breaks nothing visible until someone has a key
and a spare ten minutes, and the failure looks exactly like a configuration
problem. Everything here runs offline: the guards it covers all sit before the
first API call, which is what makes them testable at all.

    python3 -m unittest discover -s audits -v
"""

import copy
import json
import pathlib
import subprocess
import sys
import tempfile
import unittest

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))

import audit  # noqa: E402

ROOT = pathlib.Path(__file__).resolve().parent.parent
EVAL = ROOT / "audits" / "eval"


def spec_file(spec):
    path = pathlib.Path(tempfile.mkdtemp()) / "spec.json"
    path.write_text(json.dumps(spec))
    return path


class TheExtractor(unittest.TestCase):
    def setUp(self):
        self.texts, self.locations = audit.extract()

    def test_finds_public_items(self):
        self.assertGreater(len(self.locations), 20, "the workspace has more than 20 public items")
        self.assertEqual(len(self.texts), len(self.locations), "texts and locations must stay aligned")

    def test_captures_the_documentation_and_the_signature(self):
        by_name = {item["name"]: item for item in self.locations}
        build_cli = by_name["build_cli"]
        self.assertIn("serve_to", build_cli["doc"], "the doc body should survive extraction")
        self.assertIn("pub fn build_cli", build_cli["signature"])

    def test_omits_the_file_path_from_what_the_model_sees(self):
        # The path names the crate, and a crate called cmd-doctor would nudge
        # every answer about an item inside it before the item was read.
        for text in self.texts:
            self.assertNotIn("crates/", text)


class ResolvingAnEvaluationSet(unittest.TestCase):
    def setUp(self):
        self.texts, self.locations = audit.extract()
        self.good = json.loads((EVAL / "stringly-typed.json").read_text())

    def test_resolves_a_shipped_set(self):
        question, cases, texts = audit.resolve(EVAL / "stringly-typed.json", self.locations, self.texts)
        self.assertEqual(question, "stringly_typed")
        self.assertEqual(len(cases), len(texts))

    def test_refuses_an_item_that_no_longer_exists(self):
        stale = copy.deepcopy(self.good)
        stale["items"][0]["name"] = "renamed_away_by_a_refactor"
        with self.assertRaises(SystemExit):
            audit.resolve(spec_file(stale), self.locations, self.texts)

    def test_refuses_a_set_with_only_one_label(self):
        # A prompt that always answers `no` scores 100% on an all-`no` set.
        flat = copy.deepcopy(self.good)
        for case in flat["items"]:
            case["expected"] = False
        with self.assertRaises(SystemExit):
            audit.resolve(spec_file(flat), self.locations, self.texts)

    def test_accepts_a_constructed_item(self):
        made = copy.deepcopy(self.good)
        made["items"] = [
            {"as": "made", "expected": True, "text": "/// x\npub fn f(s: String) -> bool {"},
            made["items"][-1],
        ]
        _, cases, texts = audit.resolve(spec_file(made), self.locations, self.texts)
        self.assertIn("pub fn f(s: String)", texts[0])
        self.assertEqual(len(cases), 2)


class EveryShippedSet(unittest.TestCase):
    def test_every_question_has_a_baseline(self):
        asked = set(json.loads((ROOT / "audits" / "rust-canon.json").read_text()))
        scored = {json.loads(p.read_text())["question"] for p in EVAL.glob("*.json")}
        self.assertEqual(asked - scored, set(), "a question with no baseline runs unmeasured")

    def test_every_set_resolves_and_has_both_labels(self):
        texts, locations = audit.extract()
        for path in sorted(EVAL.glob("*.json")):
            with self.subTest(path.name):
                _, cases, _ = audit.resolve(path, locations, texts)
                labels = {case["expected"] for case in cases}
                self.assertEqual(labels, {True, False})

    def test_every_case_says_why_it_is_labelled_that_way(self):
        for path in sorted(EVAL.glob("*.json")):
            for case in json.loads(path.read_text())["items"]:
                with self.subTest(f"{path.name}:{case.get('name') or case.get('as')}"):
                    self.assertTrue(case.get("why", "").strip(),
                                    "a label with no stated reason cannot be reviewed")


# A stand-in for `jev classify`. It refuses to answer unless both files it was
# handed actually parse, which is the whole point: the real bug was a caller
# that passed the name of a file it was still writing.
STUB = '''import json, sys
args = sys.argv[1:]
def path_after(flag):
    return args[args.index(flag) + 1]
questions = json.load(open(path_after("--questions-file")))
items = json.load(open(path_after("--items-file")))
assert questions, "the questions file was empty when jev read it"
name = next(iter(questions))
print(json.dumps({
    "model": "stub",
    "items": [{"item": i, "answers": {name: {"type": "noul", "noul": 1.0}},
               "uncertain": False} for i in items],
    "uncertain": 0,
    "failed": 0,
}))
'''


class TheWholeEvaluationPath(unittest.TestCase):
    """Runs `run_eval` for real against a stub, with no network and no key.

    Regression: the questions file used to be handed to `jev` from inside the
    `with` block that wrote it, so the subprocess read an empty file and every
    run died on `EOF while parsing a value`. Testing `write_json` alone cannot
    catch that — CPython flushes when the handle falls out of scope, so the
    bug only appears when something reads the file while the writer is open.
    Only exercising the caller reproduces it.
    """

    def setUp(self):
        stub = pathlib.Path(tempfile.mkdtemp()) / "stub_jev.py"
        stub.write_text(STUB)
        launcher = stub.parent / "jev"
        launcher.write_text(f'#!/bin/sh\nexec {sys.executable} {stub} "$@"\n')
        launcher.chmod(0o755)
        self.original, audit.JEV = audit.JEV, str(launcher)
        self.texts, self.locations = audit.extract()

    def tearDown(self):
        audit.JEV = self.original

    def test_the_questions_file_has_been_written_before_jev_reads_it(self):
        question, cases, texts = audit.resolve(EVAL / "stringly-typed.json", self.locations, self.texts)
        # The stub raises if either file is empty, and `classify` turns a
        # non-zero exit into SystemExit. Reaching a verdict at all is the
        # regression check.
        verdict = audit.run_eval(question, cases, texts)

        # The stub answers 1.0 for every item, so the run is correct on exactly
        # the cases labelled yes. Asserting that covers the scoring too, rather
        # than only that nothing raised.
        rate = sum(1 for case in cases if case["expected"]) / len(cases)
        self.assertEqual(verdict, rate >= audit.MIN_CORRECT)
        self.assertLess(rate, audit.MIN_CORRECT,
                        "a stub that answers yes to everything must not clear the bar")

    def test_a_question_that_always_answers_yes_is_rejected(self):
        question, cases, texts = audit.resolve(EVAL / "stringly-typed.json", self.locations, self.texts)
        self.assertFalse(audit.run_eval(question, cases, texts),
                         "a constant answer must not pass the baseline")


class ReadingAnAnswer(unittest.TestCase):
    def test_reads_the_shape_the_api_actually_sends(self):
        # Captured from a real response, not invented. The first version of
        # this parser guessed at `probability` and `value` and silently
        # returned None for every answer, which surfaced as "no answer came
        # back" the first time a key was available.
        live = {"doc_restates_the_name": {"type": "noul", "noul": 0.86}}
        self.assertAlmostEqual(audit.probability(live, "doc_restates_the_name"), 0.86)

    def test_tolerates_the_other_shapes_without_relying_on_them(self):
        for answers, expected in [
            ({"q": 0.8}, 0.8),
            ({"q": {"probability": 0.3}}, 0.3),
            ({"q": {"value": 0.6}}, 0.6),
        ]:
            self.assertAlmostEqual(audit.probability(answers, "q"), expected)

    def test_reports_nothing_rather_than_guessing(self):
        self.assertIsNone(audit.probability({}, "q"))
        self.assertIsNone(audit.probability(None, "q"))
        self.assertIsNone(audit.probability({"q": "not a number"}, "q"))


if __name__ == "__main__":
    unittest.main()

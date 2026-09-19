#!/usr/bin/env python3
"""Audit this codebase with the CLI it builds.

Two phases, in this order and not the other:

  1. Score `doc_restates_the_name` against `eval/doc-restates-the-name.json`,
     whose answers were settled by reading the items. A prompt that cannot
     reproduce known answers cannot be trusted on unknown ones.
  2. Only if it clears the bar, classify every public item and report.

    audits/audit.py                 # both phases
    audits/audit.py --eval-only     # just the measurement
    audits/audit.py --skip-eval     # trust the prompt, audit anyway

Needs TYPESAFE_API_KEY, like any other caller of the API.
"""

import argparse
import json
import os
import pathlib
import subprocess
import sys
import tempfile

ROOT = pathlib.Path(__file__).resolve().parent.parent
AUDITS = ROOT / "audits"
JEV = os.environ.get("JEV", "jev")

# Below this, a run says more about the prompt than about the code.
MIN_CORRECT = 0.85


def die(message, code=1):
    print(f"audit: {message}", file=sys.stderr)
    sys.exit(code)


def extract():
    """Every public item, as (texts, locations)."""
    with tempfile.NamedTemporaryFile("r", suffix=".json", delete=False) as loc:
        out = subprocess.run(
            [sys.executable, str(AUDITS / "extract-items.py"),
             "--root", str(ROOT), "--locations", loc.name],
            capture_output=True, text=True, check=True,
        )
        return json.loads(out.stdout), json.loads(pathlib.Path(loc.name).read_text())


def classify(items, questions, concurrency=8):
    """Run `jev classify` over `items` and return its parsed report."""
    with tempfile.NamedTemporaryFile("w", suffix=".json", delete=False) as handle:
        json.dump(items, handle)
        items_file = handle.name

    result = subprocess.run(
        [JEV, "classify", "--questions-file", str(questions),
         "--items-file", items_file, "--concurrency", str(concurrency),
         "--format", "json"],
        capture_output=True, text=True,
    )
    if result.returncode != 0:
        die(f"`jev classify` failed:\n{result.stdout or result.stderr}", result.returncode)
    return json.loads(result.stdout)


def probability(answers, question):
    """A noul's probability of yes, however the answer happens to be shaped."""
    answer = (answers or {}).get(question)
    if isinstance(answer, dict):
        for key in ("probability", "value", "answer", "score"):
            if isinstance(answer.get(key), (int, float)):
                return float(answer[key])
    return float(answer) if isinstance(answer, (int, float)) else None


def run_eval(spec_path):
    """Score one question against answers settled by hand. Returns True to proceed."""
    spec = json.loads(spec_path.read_text())
    question = spec["question"]
    _, locations = extract()
    index = {(item["file"], item["name"]): item for item in locations}

    wanted, texts = [], []
    for case in spec["items"]:
        found = index.get((case["file"], case["name"]))
        if not found:
            die(f"evaluation item {case['file']}::{case['name']} no longer exists; "
                "the set is stale and scoring it would be meaningless")
        doc = found["doc"] or "(no documentation)"
        texts.append(f"/// {doc}\n{found['signature']}")
        wanted.append(case)

    single = {question: json.loads((AUDITS / "rust-canon.json").read_text())[question]}
    with tempfile.NamedTemporaryFile("w", suffix=".json", delete=False) as handle:
        json.dump(single, handle)
        report = classify(texts, handle.name)

    correct = uncertain = 0
    yes_when_true, yes_when_false = [], []
    print(f"\n  {question} — {len(wanted)} items with known answers\n")
    for case, got in zip(wanted, report["items"]):
        p = probability(got.get("answers"), question)
        if p is None:
            die(f"no answer came back for {case['name']}: {got.get('error')}")
        said = p >= 0.5
        ok = said == case["expected"]
        correct += ok
        uncertain += bool(got.get("uncertain"))
        (yes_when_true if case["expected"] else yes_when_false).append(p)
        print(f"    {'ok  ' if ok else 'MISS'}  p={p:.2f}  "
              f"expected={'yes' if case['expected'] else 'no ':<3}  {case['name']}")

    mean = lambda xs: sum(xs) / len(xs) if xs else 0.0
    separation = mean(yes_when_true) - mean(yes_when_false)
    rate = correct / len(wanted)
    print(f"\n    correct {correct}/{len(wanted)} ({rate:.0%})   "
          f"uncertain {uncertain}/{len(wanted)}   separation {separation:+.2f}")

    if rate < MIN_CORRECT:
        print(f"\n  Below {MIN_CORRECT:.0%}. The prompt is what needs work, not the code.")
        return False
    return True


def audit(questions):
    texts, locations = extract()
    report = classify(texts, questions)
    names = list(json.loads(pathlib.Path(questions).read_text()).keys())

    flagged = {name: [] for name in names}
    for item, where in zip(report["items"], locations):
        for name in names:
            p = probability(item.get("answers"), name)
            if p is not None and p >= 0.5:
                flagged[name].append((p, where, bool(item.get("uncertain"))))

    print(f"\n  {len(texts)} public items, model {report['model']}, "
          f"{report['uncertain']} uncertain, {report['failed']} failed\n")
    for name in names:
        hits = sorted(flagged[name], reverse=True, key=lambda h: h[0])
        print(f"  {name}: {len(hits)}")
        for p, where, unsure in hits:
            mark = " (uncertain)" if unsure else ""
            print(f"      {p:.2f}  {where['file']}:{where['line']}  "
                  f"{where['kind']} {where['name']}{mark}")
        print()
    print("  An answer above 0.5 is a ranking, not a verdict. Confirm by reading the\n"
          "  item before changing it, and keep every gate green through each change.")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--eval-only", action="store_true")
    parser.add_argument("--skip-eval", action="store_true")
    args = parser.parse_args()

    if not os.environ.get("TYPESAFE_API_KEY", "").strip():
        die("TYPESAFE_API_KEY is not set. `jev doctor` says the same thing at more length.")

    if not args.skip_eval:
        if not run_eval(AUDITS / "eval" / "doc-restates-the-name.json"):
            sys.exit(2)
    if not args.eval_only:
        audit(AUDITS / "rust-canon.json")


if __name__ == "__main__":
    main()

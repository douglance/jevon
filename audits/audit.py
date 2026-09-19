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


def write_json(value, suffix=".json"):
    """A closed temp file holding `value`. Closed matters: an open handle is an
    empty file to the subprocess that reads it."""
    with tempfile.NamedTemporaryFile("w", suffix=suffix, delete=False) as handle:
        json.dump(value, handle)
        return handle.name


def classify(items, questions, concurrency=8):
    """Run `jev classify` over `items` and return its parsed report."""
    items_file = write_json(items)

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


def resolve(spec_path, locations):
    """Turn one evaluation set into (question, cases, texts), or fail loudly.

    Every set is resolved before any of them is classified, so a stale or
    degenerate set costs nothing and is reported before the first API call
    rather than after an unrelated set has already been paid for.
    """
    spec = json.loads(spec_path.read_text())
    index = {(item["file"], item["name"]): item for item in locations}

    cases, texts = [], []
    for case in spec["items"]:
        # A case is either an item in this repository, or one written by hand
        # because the repository has no example of that class. Both are needed:
        # a set with only one label scores a prompt that always answers that
        # label at 100%, which measures nothing.
        if "text" in case:
            texts.append(case["text"])
        else:
            found = index.get((case["file"], case["name"]))
            if not found:
                die(f"evaluation item {case['file']}::{case['name']} no longer exists; "
                    "the set is stale and scoring it would be meaningless")
            doc = found["doc"] or "(no documentation)"
            texts.append(f"/// {doc}\n{found['signature']}")
        cases.append(case)

    if len({case["expected"] for case in cases}) < 2:
        die(f"{spec_path.name} labels only one class; such a set cannot "
            "distinguish a good prompt from a constant answer")

    return spec["question"], cases, texts


def run_eval(question, wanted, texts):
    """Score one question against answers settled by hand. Returns True to proceed."""
    single = {question: json.loads((AUDITS / "rust-canon.json").read_text())[question]}
    report = classify(texts, write_json(single))

    correct = uncertain = 0
    yes_when_true, yes_when_false = [], []
    print(f"\n  {question} — {len(wanted)} items with known answers\n")
    for case, got in zip(wanted, report["items"]):
        label = case.get("name") or case.get("as", "constructed")
        p = probability(got.get("answers"), question)
        if p is None:
            die(f"no answer came back for {label}: {got.get('error')}")
        said = p >= 0.5
        ok = said == case["expected"]
        correct += ok
        uncertain += bool(got.get("uncertain"))
        (yes_when_true if case["expected"] else yes_when_false).append(p)
        made = " *" if "text" in case else ""
        print(f"    {'ok  ' if ok else 'MISS'}  p={p:.2f}  "
              f"expected={'yes' if case['expected'] else 'no ':<3}  {label}{made}")

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
        specs = sorted((AUDITS / "eval").glob("*.json"))
        scored = {json.loads(s.read_text())["question"] for s in specs}
        asked = set(json.loads((AUDITS / "rust-canon.json").read_text()))
        if asked - scored:
            die(f"no evaluation set for {', '.join(sorted(asked - scored))}; "
                "write one or pass --skip-eval and read the output as a guess")
        _, locations = extract()
        resolved = [resolve(spec, locations) for spec in specs]
        if not all([run_eval(*one) for one in resolved]):
            sys.exit(2)
    if not args.eval_only:
        audit(AUDITS / "rust-canon.json")


if __name__ == "__main__":
    main()

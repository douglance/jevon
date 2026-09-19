# Audits

Auditing this codebase with the CLI it builds.

```sh
export TYPESAFE_API_KEY=...
audits/audit.py
```

Two phases, in this order. First the question is scored against items whose
answers were settled by reading them; only if it clears 85% does the audit run
on the 31 public items whose answers nobody knows. A prompt that cannot
reproduce known answers cannot be trusted on unknown ones, and a green run
against no baseline is a number, not a measurement.

## What these questions ask, and what they deliberately do not

The workspace already denies `unwrap_used`, `expect_used`, `panic`,
`missing_docs`, `wildcard_imports` and `too_many_arguments`, and
`cargo xtask check` caps file length, function length, nesting, arity and
cognitive complexity. Asking a model whether a function unwraps is paying for
a worse answer to a question clippy has already settled — and settled
exactly, which a probability never is.

So every question here targets the gap: things that pass every gate and are
still not canonical Rust.

| Question | What it catches that no lint can |
|---|---|
| `doc_restates_the_name` | `missing_docs` proves a doc exists. It cannot read it. |
| `stringly_typed` | A `&str` that stands for four alternatives type-checks fine. |
| `owns_what_it_could_borrow` | Clippy catches some; the caller-facing ones need judgment about intent. |
| `signature_leaks_internals` | Whether a type is part of the promise or of the implementation is a design question. |

Each is one noul rather than a choice over quality labels. That is not taste:
the sibling audit in the SDK repository measured four wordings against eleven
tests whose strength had been settled by mutation, and overlapping choice
options spread probability across labels that looked confident and were not,
while a single falsifiable yes/no separated the classes far better. The same
write-up records that adding a worked example to a criterion made results
worse, because the model anchored on the example's surface form. There are no
examples in the criteria here for that reason.

## What was measured, including what failed

These are results, not intentions. Each row is a scored run against answers
settled by reading the items first.

| Question | Correct | Separation | |
|---|---|---|---|
| `doc_restates_the_name` | 12/12 | +0.57 | kept |
| `owns_what_it_could_borrow` | 8/9 | +0.63 | kept |
| `stringly_typed` | 7/8 | +0.47 | kept |
| `signature_leaks_internals` | 4/7 | **-0.02** | **dropped** |

`signature_leaks_internals` asked whether a signature names a type belonging to
how an item is built rather than to what it promises. It answered about 0.2 to
every item in both classes, across three runs. It was not short of
information — with the file's imports in view it still said no. Reading its
answers back, it was right and the labels were wrong: for a CLI built on a
framework, returning that framework's type *is* the promise. A question that
cannot separate its classes produces numbers that look like findings, so it
was removed rather than reworded.

Two experiments that seemed obvious and made things worse:

- **Giving items their imports and bodies.** The theory was that
  `signature_leaks_internals` could not attribute `-> Cli` without the `use`
  lines. It moved that question by 0.01 and dropped
  `doc_restates_the_name` from 14/14 at +0.63 to 10/14 at +0.20, because a doc
  that only restates a name stops looking like one when a body sits under it.
  Reverted. More evidence is not more signal.
- **Handing over more context generally.** The sibling audit in the SDK
  repository found the same shape: an example added to a criterion made results
  worse, because the model anchored on the example's surface form.

The first run also found a defect in this tooling rather than in the code: the
answer parser guessed at key names and never matched the real shape, which is
`{"type": "noul", "noul": 0.86}`. Every answer came back as None. It was
written from imagination and fixed by looking.

## The evaluation set

`eval/doc-restates-the-name.json` holds fourteen items, six labelled yes and
eight no, each with the reason it was labelled that way. They are named by file
and item rather than by index, so reordering the extractor cannot silently
re-point a label at a different item; if one stops resolving, the run fails
rather than scoring a stale set.

Only `doc_restates_the_name` has a baseline so far. The other three questions
run unscored, which means their output is a suggestion and should be read as
one until each has a set of its own.

## Reading the output

An answer above 0.5 is a ranking, not a verdict. Confirm by reading the item
before changing anything. The classifier sees one item at a time with no
knowledge of the rest of the codebase, so it cannot tell a leaky signature from
a deliberate one, and it will flag a type that is public precisely because
callers are meant to depend on it.

## In CI

The `audit` job runs both phases on every push, using the `jev` built from
that same commit, so the tool and the code it judges are never different
revisions. The baseline is a hard check — a reworded question that stops
reproducing answers settled by reading is a regression a machine can catch.
The audit itself only reports, because failing a build on a probability would
teach everyone to route around it.

Without `TYPESAFE_API_KEY` in the repository secrets the job says so and
passes. An audit that cannot run is not a failure; a permanently red job
everyone learns to ignore would be.

Every gate must be green before and after each change:

```sh
cargo fmt --all --check
cargo xtask check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --locked
```

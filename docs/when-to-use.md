# When jev fits

## The precondition

Jev is useful exactly when your program already has a decision to make, the
options are known in advance, and judgment is the hard part. It does not
generate anything.

The test is mechanical: **if you can write the `match` arms before the call, it
fits. If you can't, it doesn't.**

```rust
// Fits. The arms exist before the answer does.
match Ticket::answers(&response)?.category {
    TicketCategory::Billing => route_to_finance(),
    TicketCategory::Technical => route_to_engineering(),
    TicketCategory::Other => triage(),
}
```

Everything below follows from that one sentence. If a situation here sounds
like yours but you cannot name the options up front, the precondition has not
been met and the fit is not there.

## Where these numbers come from

The shares below were produced by classifying public projects that use the API
and then hand-checking the result. Read them as shape, not as measurement:

- Among confident labels (probability ≥ 0.8), 6 of 7 spot-checked were correct.
- The incorrect ones confused **routing** with **picking an action in a loop**,
  which are adjacent and easy to conflate. Treat the boundary between those two
  as soft.
- Low-confidence rows were genuinely uninformative source material, not model
  failure. This matters: a thin input is the usual reason an answer is
  unconfident, here and in your own use.

**The shape is sound. The percentages are approximate.** They are here to show
relative weight, not to be quoted.

## The seven situations

| # | Situation | Share |
|---|---|---|
| 1 | Pick the next action in a loop | ~45% |
| 2 | Filter or rank a stream | ~17% |
| 3 | Gate — pass or fail against rules | ~16% |
| 4 | Label or score a large batch | ~10% |
| 5 | Route between tools, models, agents or branches | ~9% |
| 6 | Extract structured fields from messy text | ~3% |
| 7 | Score against a rubric | — |

### 1. Pick the next action in a loop (~45%)

Game moves, robot steps, browser actions, trading decisions, test steps. You
hand it the state and the legal actions; it picks one.

The schema is doing the important work here: it makes an illegal move
*impossible* rather than unlikely. Tetris, MuJoCo arms, Stagehand browser
control and RuneScape bots all have this shape. Budget around 200–400ms per
step.

### 2. Filter or rank a stream (~17%)

Feed filtering, ad blocking, inbox triage, engagement-bait detection, pruning
an agent's context. Everything arrives, most of it should be dropped, and the
question is the same every time.

### 3. Gate — pass or fail against rules (~16%)

A verdict before something proceeds: PR checks, draft quality gates, content
moderation, commit review, reward-hacking guards, "is a safeguard needed here".

The output is a boolean *and* a probability, so you set the threshold yourself
and route the uncertain cases to a human instead of acting on a coin flip.

### 4. Label or score a large batch (~10%)

Emails, résumés, ads, articles, support tickets, replay events. This is where
cost per item dominates rather than latency per call — 2,225 articles in 4.9
seconds, 63,045 emails in under three minutes.

### 5. Route between tools, models, agents or branches (~9%)

Which skill, which model, which component, which handler. Cheap enough to put
in front of an expensive call: several projects use it to decide whether a full
LLM is needed at all.

### 6. Extract structured fields from messy text (~3%)

Mapping fields between schemas, parsing a query into filters, walking an
ontology. Done as a set of choice questions rather than as free-form
extraction, which is what keeps the output shape guaranteed.

### 7. Score against a rubric

Any time you want a position on a scale rather than a bucket. The score comes
back continuous — 1.03, 1.95 — so you can sort by it.

Rounding it to a level throws away the only property that makes ranking work.
If you find yourself writing `round(score)`, you wanted a choice question.

## When it is the wrong tool

**You need text, code or prose out.** It does not generate. Nothing across the
projects surveyed uses it to produce anything.

**The options are not known ahead of time.** Open-ended output is the other
kind of model's job.

**One call, no latency pressure.** If you already have an LLM in the loop and
volume is low, adding a second vendor buys nothing.

**The input is thin.** A bare filename or ID cannot be judged, however good the
question is. This is the dominant cause of low-confidence answers — reach for
more context before reaching for a better prompt.

**You need the reasoning.** You get a probability, not an argument.

**Your labels overlap.** Forced into one of 11 overlapping categories, top-1
agreement with human curators was 59% — but the right label was in the top 3
for 92%. When your categories are not crisply disjoint, use it for ranking and
thresholds, never for a single forced label.

## When it actually pays

Four conditions. The more that hold, the better the fit:

1. **The decision repeats**, so connection and prompt cost amortise.
2. **Your budget is over ~200ms and under ~500ms.** Tighter and you want a
   local model; looser and a general LLM will do.
3. **Per-decision cost matters**, because volume is high.
4. **You want probabilities rather than a label**, so you can defer the
   uncertain cases.

## Do not shell out per decision

Repeated calls should reuse one connection. Spawning the binary per decision
pays for a TLS handshake every time.

Measured on one machine, eight items, single run — enough to show the shape,
not a benchmark:

| How | Per decision |
|---|---|
| One `jev ask` process per item | 326ms |
| One `jev classify`, `--concurrency 1` | 206ms |
| One `jev classify`, `--concurrency 8` | 48ms |

Reusing the connection is worth about 1.6×. Having several requests in flight
is worth another 4× on top of that, and it is the larger effect for anything
batch-shaped. Neither helps a genuinely one-off call.

From a program, use the SDK
([typesafe-sdk-rs](https://github.com/douglance/typesafe-sdk-rs)) and hold one
`Client`.

From the command line, send the whole batch to `classify` rather than looping
in a shell — one client, one connection, several items in flight at once:

```sh
# One process, one connection, N decisions.
jev classify --noul "Is this a bug report?" --concurrency 8 < titles.txt

# Not this: one process and one TLS handshake per line.
while read -r line; do jev ask "$line" --noul "..."; done < titles.txt
```

`--items-file` takes a JSON array when an item contains newlines.

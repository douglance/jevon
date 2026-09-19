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

491 unique public projects, merged from two independent sources: a gallery of
posts about the API, and a curated list of repositories checked against their
own source. **280 of them actually call Jev to decide something** — the rest
reimplement the model, benchmark it, wrap it as an SDK, or list it. The shares
below are over the 210 whose shape was confidently labelled.

Read them as shape, not as measurement:

- The two sources disagree sharply on weight. Posts reward what you can watch,
  so control loops are 76 of them against 14 repositories; repositories reward
  what you can install, so routing and gating are far better represented there.
  Neither population alone gives the right prior.
- **Routing and picking an action in a loop are adjacent and easy to conflate.**
  Treat that boundary as soft; it is where the hand-checked errors were.
- Low-confidence rows were uninformative source material, not model failure. A
  thin input is the usual reason an answer is unconfident, here and in your own
  use.

**The shape is sound. The percentages are approximate.** They show relative
weight and are not for quoting.

## The six situations

| # | Situation | Share |
|---|---|---|
| 1 | Pick the next action in a loop | ~43% |
| 2 | Filter or rank a stream | ~20% |
| 3 | Route between tools, models, agents or branches | ~15% |
| 4 | Gate — pass or fail against rules | ~11% |
| 5 | Label or score a large batch | ~6% |
| 6 | Assemble an artefact from a sequence of choices | ~4% |

Extracting structured fields is a seventh thing people try, and across all 491
projects **not one instance was confidently labelled**. It works — it is how
field mapping and query parsing are done — but nobody has built much with it.
Treat it as unexplored rather than proven.

### 1. Pick the next action in a loop (~43%)

Game moves, robot steps, browser actions, trading decisions, test steps. You
hand it the state and the legal actions; it picks one.

The schema is doing the important work: it makes an illegal move *impossible*
rather than unlikely. Tetris, MuJoCo arms, Stagehand browser control, a drone
at 2.5Hz and one trade per block all have this shape. Budget around 200–400ms
per step.

The ones that work read **structured state**, not pixels.

### 2. Filter or rank a stream (~20%)

Feed filtering, ad blocking, inbox triage, engagement-bait detection, résumé
and job matching, support-ticket triage. Everything arrives, most of it should
be dropped, and the question is the same every time.

The standout instance is **pruning an agent's own context** — scoring every
past tool call and result in one request and dropping the stale ones. Several
people built that independently, in both sources, and it has more adoption than
anything else here.

### 3. Route between tools, models, agents or branches (~15%)

Which skill, which model, which component, which handler. Cheap enough to put
in front of an expensive call: several projects use it to decide whether a full
LLM is needed at all.

This is the shape that gets **embedded in other people's tools** rather than
shipped standalone — model routers for coding agents, skill selection, semantic
HTTP routing. If you want the work to be adopted rather than admired, it is the
most promising of the six.

### 4. Gate — pass or fail against rules (~11%)

A verdict before something proceeds: PR checks, draft quality gates, content
moderation, tool-permission checks, reward-hacking guards.

The output is a boolean *and* a probability, so you set the threshold yourself
and route the uncertain cases to a human instead of acting on a coin flip.

Worth knowing before you build one: gates are **built often and adopted
rarely**. Everyone wants their own rules, so a gate is usually worth writing
for yourself and rarely worth publishing.

### 5. Label or score a large batch (~6%)

Emails, résumés, ads, articles, support tickets, replay events. This is where
cost per item dominates rather than latency per call — 2,225 articles in 4.9
seconds, 63,045 emails in under three minutes.

### 6. Assemble an artefact from a sequence of choices (~4%)

Music from spoken instructions, platformer levels, a page composed per reader,
3D character expressions, an image predicted a region at a time.

This one looks like it contradicts "it does not generate anything", and it does
not: the model still only ever picks from options you defined. Your code does
the assembling. It is listed separately because people reach for it without
recognising it as a decision problem.

## Picking the primitive

Orthogonal to all six. Once you know the shape, the primitive follows from what
the answer *means*:

- **noul** — whether a condition holds. The probability is the answer; there is
  no separate confidence.
- **choice** — one of a set you defined. You get the pick and the full
  distribution.
- **score** — a position along an ordered rubric. The value comes back
  continuous — 1.03, 1.95 — so you can sort by it.

Rounding a score to a level throws away the only property that makes ranking
work. If you find yourself writing `round(score)`, you wanted a choice.

## When it is the wrong tool

**You need text, code or prose out.** It does not generate, and across 491
projects nothing uses it to. Situation 6 is not an exception: there the model
picks from options you defined and your code assembles the result.

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
jev classify --stdin --noul "Is this a bug report?" --concurrency 8 < titles.txt

# Not this: one process and one TLS handshake per line.
while read -r line; do jev ask "$line" --noul "..."; done < titles.txt
```

Say where the items come from: `--stdin` for one per line, `--items` to pass
them directly, or `--items-file` for a JSON array when an item contains
newlines. Standard input is never read unless you ask for it, because over MCP
that stream carries the protocol.

## Before you trust a question

A question that reads well can still be worthless, and nothing about the
answers tells you which kind you have. Measure it against items whose answers
you already know:

```sh
jev eval --labels-file graded.json \
  --noul "Is this urgent?" \
  --noul "Does this need attention today?"
```

Read `auc` — the chance a true item outranks a false one — and treat it as the
only portable number. Rewording moves every individual probability, so a
threshold tuned against one phrasing is wrong against the next. Repeat `--noul`
to rank several wordings in one run.

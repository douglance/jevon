# jevon

`jev` — the command-line interface and MCP server for the [TypeSafe](https://typesafe.ai) API.

Ask questions about a piece of text and get answers with probabilities
attached, instead of a string you have to parse.

```sh
cargo install jevon
export TYPESAFE_API_KEY=...

jev ask "I was charged twice, please fix this" \
  --noul "What is this ticket about?" \
  --choice billing --choice technical --choice other
```

Looking for the Rust library rather than the binary? That is
[typesafe-sdk-rs](https://github.com/douglance/typesafe-sdk-rs). This repository
depends on it from crates.io like any other consumer.

## Commands

| Command | What it does |
|---|---|
| `jev ask` | Ask questions about one piece of text. |
| `jev classify` | Apply one question set to many items, several in flight at once. |
| `jev doctor` | Report the resolved configuration and anything that would stop it working. |
| `jev models list` | Inspect the models available to this account. |

Three question primitives, picked by what the answer means:

- `--choice` — one of a defined set. Include a no-match label when none may apply.
- `--score` — a degree along an ordered dimension, lowest level first.
- `--noul` — whether a condition holds, returned as a probability of yes.

Independent questions sent in one `--questions` object are answered in parallel
and cost one round trip:

```sh
jev ask "$TICKET" --questions '{
  "category": {"type":"choice","instructions":"What is this about?",
               "criteria":{"billing":null,"technical":null,"other":null}},
  "urgent":   {"type":"noul","instructions":"Is it urgent?"}
}'
```

`classify` takes the list on stdin, so a batch is one client and one connection
rather than a shell loop:

```sh
jev classify --noul "Is this a bug report?" --min-confidence 0.7 < titles.txt
```

`uncertain` counts answers below `--min-confidence`. Those are the rows worth
reading rather than acting on, and usually mean the item carried too little
context to judge.

## Agents

One command graph serves every surface, so a command added here reaches all of
them at once:

```sh
jev mcp add             # register the binary as an MCP server
jev skills add          # write a skill file per command
jev completions zsh     # shell completions
jev ask --schema        # JSON Schema for the command's arguments and options
jev --llms-full         # the whole manifest, with worked examples
```

Every command is annotated read-only, so an MCP client does not route a
harmless read through an approval prompt. Output shape is chosen with
`--format toon|json|yaml|md|jsonl`, and `--token-count`, `--token-limit`,
`--token-offset` and `--filter-output` exist so an agent can stay inside a
context window.

`jev mcp add` writes no environment. The agent must already have
`TYPESAFE_API_KEY` in scope — deliberately, because a key does not belong in an
agent config file. Without one, every call returns "No API key was provided"
rather than failing obscurely.

The binary also installs as `typesafe`, the former name, so anything already
written against it keeps working.

## Configuration

| Variable | Default | Purpose |
|---|---|---|
| `TYPESAFE_API_KEY` | — | Required. |
| `TYPESAFE_BASE_URL` | `https://api.typesafe.ai` | API root. |
| `TYPESAFE_DEFAULT_MODEL` | `jev-latest` | Model used when a request names none. |
| `TYPESAFE_LOG_LEVEL` | `warn` | `debug`, `info`, `warn`, `error`, `off`. |

An explicit argument wins over the environment, which wins over the default. A
variable set to whitespace counts as unset. Run `jev doctor` to see what
resolved and what is missing; it is the only command that needs no network.

Retry, timeout and backoff behaviour belongs to the SDK and is documented
there.

## Layout

Crates are layered, and a crate may depend only on its own layer or below. The
table lives in `xtask/src/layers.rs`, and a crate missing from it fails the
build.

```
0  cmd-kit                                    arguments, client, output shaping
1  cmd-ask · cmd-classify · cmd-doctor · cmd-models
2  jevon (the `jev` binary) · xtask
```

The SDK is a published dependency, not a workspace member. That is the point of
the split: the commands are held to the API TypeSafe actually ships, so a reach
into a private corner of the SDK fails here rather than after a release.

Only `cmd-kit` may name `typesafe-sdk-http`, and no crate here may name
`reqwest` or `hyper`. One crate builds the transport, so retry and timeout
behaviour cannot quietly diverge between two code paths.

## Development

```sh
cargo fmt --all --check
cargo xtask check      # file and function size, complexity, layering
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

All four must pass. `cargo xtask check` is the authority on structure: review
does not reliably notice that a file crossed 150 lines or that a command crate
acquired a socket. Files are capped at 150 lines and functions at 30, with
cognitive complexity 7 — an escape hatch exists but must name the metric and
give a reason:

```rust
// typesafe-allow file-length reason: the layer map reads best as one table.
```

Nothing here talks to the API. The command graph is driven in-process through
`serve_to`, which is the same path the binary takes, so a test cannot pass while
the binary breaks. Live conformance against TypeSafe runs in the SDK
repository, which is where the wire types live.

## License

MIT

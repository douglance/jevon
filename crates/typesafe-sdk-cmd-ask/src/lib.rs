//! The `ask` command.
//!
//! Questions arrive as JSON at runtime, so this uses the dynamic question
//! vocabulary rather than the typed builders. The convenience flags exist
//! because the common case — one choice or one score over a piece of text —
//! should not require hand-writing a JSON object.

mod parse;

use incurs::command::{CommandDef, Example, TypedContext, TypedResult};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use typesafe_sdk_client::SystemOneRequest;
use typesafe_sdk_cmd_kit::{Usage, client, code_for, read_only_remote};

/// What to ask about.
#[derive(Deserialize, incurs::Args)]
pub struct Args {
    /// The text to ask about. `-` reads it from standard input, which only
    /// works from a terminal — over MCP that stream carries the protocol, so
    /// pass the text itself or use `--state-file`.
    pub state: String,
}

/// How to ask.
///
/// Every field defaults, because an absent flag must parse rather than fail:
/// the CLI's job is to report "no questions were given" in its own words, not
/// to surface a deserialization error about a missing field.
#[derive(Default, Deserialize, incurs::Options)]
#[serde(default)]
pub struct Options {
    /// Questions as a JSON object keyed by answer name. Independent questions
    /// sent together are answered in parallel and cannot see each other.
    #[incurs(alias = "q")]
    pub questions: Option<String>,
    /// Read the question set from this file instead, or `-` for stdin.
    pub questions_file: Option<String>,
    /// Choose one of these labels. Probabilities compare the options, so
    /// include a no-match label when none may apply.
    #[incurs(alias = "c")]
    pub choice: Vec<String>,
    /// Rate against these ordered levels, lowest first. Each level must
    /// describe a concrete situation and stand on its own.
    #[incurs(alias = "s")]
    pub score: Vec<String>,
    /// The question to judge. Answered as a probability of yes when used
    /// alone, with no separate confidence.
    #[incurs(alias = "n")]
    pub noul: Option<String>,
    /// Read the text from this file instead of the argument. Use it when the
    /// text is long enough to hit an argument-length limit, or contains
    /// quoting a shell would mangle.
    pub state_file: Option<String>,
    /// The model to use; defaults to the configured one.
    #[incurs(alias = "m")]
    pub model: Option<String>,
}

/// The answers, as the CLI reports them.
#[derive(Serialize, JsonSchema)]
pub struct Answered {
    /// The model that answered, resolved to a concrete version.
    pub model: String,
    /// The answers, keyed by question name.
    pub answers: serde_json::Value,
    /// Tokens consumed.
    pub usage: Usage,
}

/// The `ask` command, as a definition rather than an execution.
///
/// Returned rather than run so one graph can serve the terminal, MCP,
/// `--schema`, `--llms-full` and the skill files from the same source. A
/// command that printed here would reach exactly one of them.
#[must_use]
pub fn command() -> CommandDef {
    CommandDef::typed::<Args, Options, (), Answered, _, _>(
        "ask",
        |ctx: TypedContext<Args, Options, ()>| async move {
            match run(&ctx.args, &ctx.options).await {
                Ok(answered) => TypedResult::ok(answered),
                Err(error) => TypedResult::error(code_for(&error), error.to_string()),
            }
        },
    )
    .description(
        "Ask questions about text and get typed answers with probabilities. \
         Use when code needs a judgment — routing, classification, ranking, \
         extraction or verification — rather than generated prose",
    )
    .hint(HINT)
    .examples(examples())
    .mcp(read_only_remote("Ask questions about text"))
    .done()
}

/// Guidance an agent needs that the schema cannot carry.
///
/// Rendered into the skill file as a single blockquote, so it stays one
/// paragraph. What goes here is what someone gets wrong on their first
/// integration, not what `--help` already says.
///
/// It ends by pointing at the live docs. Every command here is a call to a
/// remote service, so an agent that can run this command can also read them,
/// and they carry the current guidance this paragraph only summarises.
const HINT: &str = "Pick the primitive by what the answer means: --choice for one of a \
defined set, --score for a degree along an ordered dimension, --noul for whether a \
condition holds. Put the judgment in the question and the possible answers in the \
labels; answer names are for your code and are never shown to the model, so each \
question must carry its full meaning. Give it enough state to answer — quote the source \
text rather than summarising it. Ask every independent question in one --questions \
object: they run in parallel and cost one round trip. Read the numbers carefully: a noul \
near 0.5 means the model finds yes and no equally likely, not a medium amount of the \
thing; confidence on a choice or score reports how concentrated the distribution is, not \
whether the answer is correct. Typed output guarantees the shape, never the truth. The live docs are the source of \
truth and worth reading before a first integration: start at \
https://docs.typesafe.ai/llms.txt, then the page for the primitive you chose \
(https://docs.typesafe.ai/primitives/choice.md, /noul.md or /score.md), \
https://docs.typesafe.ai/concepts/state.md for what to put in the question, and \
https://docs.typesafe.ai/confidence.md before you act on a threshold. Append .md to any \
docs path to read it as Markdown.";

/// Worked invocations, rendered into the skill file.
fn examples() -> Vec<Example> {
    vec![route(), rate(), judge(), combined()]
}

fn example(command: &str, description: &str) -> Example {
    Example {
        command: command.to_owned(),
        description: Some(description.to_owned()),
    }
}

fn route() -> Example {
    example(
        "\"I was charged twice\" --noul \"What is this about?\" \
         --choice billing --choice technical --choice other",
        "Route a ticket by choosing one label",
    )
}

fn rate() -> Example {
    example(
        "\"The build has been broken for three days\" --noul \"How urgent is this?\" \
         --score \"can wait\" --score \"this week\" --score today",
        "Rate against an ordered rubric",
    )
}

fn judge() -> Example {
    example(
        "\"$(cat review.txt)\" --noul \"Does this mention a security problem?\"",
        "Judge whether one condition holds",
    )
}

fn combined() -> Example {
    example(
        "\"$TICKET\" --questions '{\"category\":{\"type\":\"choice\",\
         \"instructions\":\"What is this about?\",\
         \"criteria\":{\"billing\":null,\"technical\":null,\"other\":null}},\
         \"urgent\":{\"type\":\"noul\",\"instructions\":\"Is it urgent?\"}}'",
        "Ask several independent questions in one request; they run in parallel",
    )
}

/// The text to ask about: the file when one was named, otherwise the argument.
///
/// `-` still reads standard input, because from a terminal that is the natural
/// way to pipe something in. It is a terminal affordance only — over MCP that
/// stream carries the protocol.
fn state_of(args: &Args, options: &Options) -> Result<String, typesafe_sdk_error::Error> {
    match options.state_file.as_deref() {
        Some(path) => typesafe_sdk_cmd_kit::read_file(path),
        None => typesafe_sdk_cmd_kit::text(&args.state),
    }
}

async fn run(args: &Args, options: &Options) -> Result<Answered, typesafe_sdk_error::Error> {
    let questions = parse::questions_from(options)?;
    let state = state_of(args, options)?;
    let mut request = SystemOneRequest::new(state.as_str(), questions);
    if let Some(model) = options.model.clone() {
        request = request.model(model);
    }

    let response = client()?.system_one(request).await?;
    Ok(Answered {
        model: response.model,
        answers: serde_json::to_value(&response.answers).unwrap_or(serde_json::Value::Null),
        usage: Usage::from(&response.usage),
    })
}

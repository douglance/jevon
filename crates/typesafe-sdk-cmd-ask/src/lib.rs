//! The `ask` command.
//!
//! Questions arrive as JSON at runtime, so this uses the dynamic question
//! vocabulary rather than the typed builders. The convenience flags exist
//! because the common case — one choice or one score over a piece of text —
//! should not require hand-writing a JSON object.

mod parse;

use incurs::command::{CommandDef, TypedContext, TypedResult};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use typesafe_sdk_client::SystemOneRequest;
use typesafe_sdk_cmd_kit::{client, code_for, read_only_remote};

/// What to ask about.
#[derive(Deserialize, incurs::Args)]
pub struct Args {
    /// The text to ask about.
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
    /// Questions as a JSON object, keyed by answer name.
    #[incurs(alias = "q")]
    pub questions: Option<String>,
    /// Shorthand for one choice question over these labels.
    #[incurs(alias = "c")]
    pub choice: Vec<String>,
    /// Shorthand for one score question over this ordered rubric.
    #[incurs(alias = "s")]
    pub score: Vec<String>,
    /// Shorthand for one yes/no question.
    #[incurs(alias = "n")]
    pub noul: Option<String>,
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

/// Tokens consumed by the request.
#[derive(Serialize, JsonSchema)]
pub struct Usage {
    /// Input tokens used.
    pub input_tokens: u64,
    /// Output tokens used.
    pub output_tokens: u64,
}

/// Builds the `ask` command.
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
    .description("Ask questions about a piece of text and get typed answers")
    .mcp(read_only_remote("Ask questions about text"))
    .done()
}

async fn run(args: &Args, options: &Options) -> Result<Answered, typesafe_sdk_error::Error> {
    let questions = parse::questions_from(options)?;
    let mut request = SystemOneRequest::new(args.state.as_str(), questions);
    if let Some(model) = options.model.clone() {
        request = request.model(model);
    }

    let response = client()?.system_one(request).await?;
    Ok(Answered {
        model: response.model,
        answers: serde_json::to_value(&response.answers).unwrap_or(serde_json::Value::Null),
        usage: Usage {
            input_tokens: response.usage.input_tokens,
            output_tokens: response.usage.output_tokens,
        },
    })
}

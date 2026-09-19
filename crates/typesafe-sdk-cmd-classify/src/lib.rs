//! The `classify` command.
//!
//! `ask` answers about one piece of text. Classifying a list with it means one
//! process, one TLS handshake and one round trip per item, driven by a shell
//! loop — which is what sending thirteen windows through it actually looked
//! like. This applies one question set to many items over a single client,
//! several at a time, and reports which answers were not confident.

mod asking;
mod fold;
mod report;
mod run;

use incurs::command::{CommandDef, Example, TypedContext, TypedResult};
use serde::Deserialize;
use typesafe_sdk_cmd_kit::{code_for, read_only_remote};

pub use report::{Classified, Item};

/// How to classify.
#[derive(Default, Deserialize, incurs::Options)]
#[serde(default)]
pub struct Options {
    /// Questions as a JSON object, applied to every item.
    #[incurs(alias = "q")]
    pub questions: Option<String>,
    /// Read the question set from this file instead, or `-` for stdin.
    pub questions_file: Option<String>,
    /// Choose one of these labels. Shorthand for a single choice question.
    #[incurs(alias = "c")]
    pub choice: Vec<String>,
    /// Rate against these ordered levels, lowest first.
    #[incurs(alias = "s")]
    pub score: Vec<String>,
    /// The question to ask about every item.
    #[incurs(alias = "n")]
    pub noul: Option<String>,
    /// The items to classify, passed directly. The only source available over
    /// MCP, where standard input carries the protocol rather than data.
    #[incurs(alias = "i")]
    pub items: Vec<String>,
    /// Read items from this JSON array file, or `-` for that array on stdin.
    /// Use it when an item contains newlines.
    pub items_file: Option<String>,
    /// Read items from standard input, one per line.
    pub stdin: bool,
    /// How many items to have in flight at once.
    #[incurs(alias = "j", default = 8)]
    pub concurrency: u32,
    /// Mark an answer uncertain below this confidence, from 0 to 1.
    #[incurs(default = 0.5)]
    pub min_confidence: f64,
    /// The model to use; defaults to the configured one.
    #[incurs(alias = "m")]
    pub model: Option<String>,
}

/// The `classify` command, as a definition rather than an execution.
///
/// Shares `ask`'s question vocabulary on purpose: the same flags mean the
/// same thing whether one item or ten thousand are being judged.
#[must_use]
pub fn command() -> CommandDef {
    CommandDef::typed::<(), Options, (), Classified, _, _>(
        "classify",
        |ctx: TypedContext<(), Options, ()>| async move { run(&ctx.options).await },
    )
    .description("Apply one question set to many items and report which answers were not confident")
    .examples(examples())
    .mcp(read_only_remote("Classify a list of items"))
    .hint(
        "Say where the items come from: --items for a list, --items-file for a JSON \
         array, or --stdin for one per line. Standard input is never read unless \
         --stdin is passed, because over MCP that stream carries the protocol and \
         reading it hangs the call. Send the whole list in one invocation rather \
         than looping in a shell: one client, one connection, several items in \
         flight at once. Every item is answered independently, \
         so nothing an item says can influence another. `uncertain` counts answers below \
         --min-confidence, which defaults to 0.5; those are the rows worth reading rather \
         than acting on, and usually mean the item carried too little context to judge, not \
         that the model failed. Give each line enough to go on — a bare identifier cannot be \
         classified, however good the question is. The run exits 1 when nothing was \
         answered and 2 when some items failed, so a broken key cannot pass for success.",
    )
    .done()
}

/// Runs the command and picks the exit code the result deserves.
async fn run(options: &Options) -> TypedResult<Classified> {
    match run::classify(options).await {
        Ok(result) => match result.exit_code() {
            Some(code) => TypedResult::ok_with_exit_code(result, code),
            None => TypedResult::ok(result),
        },
        Err(error) => TypedResult::error(code_for(&error), error.to_string()),
    }
}

/// Worked invocations, rendered into the skill file.
fn examples() -> Vec<Example> {
    vec![
        Example {
            command: "--stdin --choice bug --choice feature --choice question \
                      --noul \"What kind of issue is this?\" < titles.txt"
                .to_owned(),
            description: Some("Label every line of a file".to_owned()),
        },
        Example {
            command: "--items \"first ticket\" --items \"second ticket\" \
                      --noul \"Is this urgent?\""
                .to_owned(),
            description: Some("Pass items directly — the only way in over MCP".to_owned()),
        },
    ]
}

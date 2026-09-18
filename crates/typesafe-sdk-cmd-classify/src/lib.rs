//! The `classify` command.
//!
//! `ask` answers about one piece of text. Classifying a list with it means one
//! process, one TLS handshake and one round trip per item, driven by a shell
//! loop — which is what sending thirteen windows through it actually looked
//! like. This applies one question set to many items over a single client,
//! several at a time, and reports which answers were not confident.

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
    /// Read items from this JSON array file instead of stdin lines, or `-`
    /// for that array on stdin. Use it when an item contains newlines.
    pub items_file: Option<String>,
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

/// Builds the `classify` command.
#[must_use]
pub fn command() -> CommandDef {
    CommandDef::typed::<(), Options, (), Classified, _, _>(
        "classify",
        |ctx: TypedContext<(), Options, ()>| async move {
            match run::classify(&ctx.options).await {
                Ok(result) => TypedResult::ok(result),
                Err(error) => TypedResult::error(code_for(&error), error.to_string()),
            }
        },
    )
    .description(
        "Apply one question set to many items read from stdin, one per line, and report \
         which answers were not confident",
    )
    .examples(examples())
    .mcp(read_only_remote("Classify a list of items"))
    .hint(
        "Send the list in on stdin rather than looping in a shell: one client, one \
         connection, several items in flight at once. Every item is answered independently, \
         so nothing an item says can influence another. `uncertain` counts answers below \
         --min-confidence, which defaults to 0.5; those are the rows worth reading rather \
         than acting on, and usually mean the item carried too little context to judge, not \
         that the model failed. Give each line enough to go on — a bare identifier cannot be \
         classified, however good the question is.",
    )
    .done()
}

/// Worked invocations, rendered into the skill file.
fn examples() -> Vec<Example> {
    vec![
        Example {
            command: "--choice bug --choice feature --choice question \
                      --noul \"What kind of issue is this?\" < titles.txt"
                .to_owned(),
            description: Some("Label every line of a file".to_owned()),
        },
        Example {
            command: "--questions-file questions.json --concurrency 16 < items.txt".to_owned(),
            description: Some("Ask a larger question set, sixteen at a time".to_owned()),
        },
    ]
}

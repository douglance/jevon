//! The `eval` command.
//!
//! A question that reads well can still be worthless, and nothing about the
//! answers says which kind you have. The only way to tell is to ask it of items
//! whose answers are already known and look at how it orders them.
//!
//! This exists because rewording one question moved its AUC from 0.87 to 0.997
//! while every absolute probability moved the other way. Wording is the largest
//! lever there is, and it cannot be pulled without a measurement.

mod labels;
mod measure;
mod report;
mod run;
mod score;
mod spread;
mod warn;

use incurs::command::{CommandDef, Example, TypedContext, TypedResult};
use serde::Deserialize;
use typesafe_sdk_cmd_kit::{code_for, read_only_remote};

pub use report::{Cut, Evaluated, Measured};

/// What to measure, and against what.
#[derive(Default, Deserialize, incurs::Options)]
#[serde(default)]
pub struct Options {
    /// A JSON array of `{"item": "…", "label": …}`, or `-` for stdin. The
    /// label is `true`/`false` for a yes/no question, or one of the labels for
    /// a choice.
    #[incurs(alias = "l")]
    pub labels_file: String,
    /// The question to measure. Repeat it to compare wordings against the same
    /// items in one run — wording is the largest lever there is.
    #[incurs(alias = "n")]
    pub noul: Vec<String>,
    /// Measure a choice between these labels instead of a yes/no question.
    #[incurs(alias = "c")]
    pub choice: Vec<String>,
    /// How many items to have in flight at once.
    #[incurs(alias = "j", default = 8)]
    pub concurrency: u32,
    /// Count a choice as confident at or above this, from 0 to 1.
    #[incurs(default = 0.5)]
    pub min_confidence: f64,
    /// The model to measure; defaults to the configured one.
    #[incurs(alias = "m")]
    pub model: Option<String>,
}

/// The `eval` command, as a definition rather than an execution.
#[must_use]
pub fn command() -> CommandDef {
    CommandDef::typed::<(), Options, (), Evaluated, _, _>(
        "eval",
        |ctx: TypedContext<(), Options, ()>| async move { run(&ctx.options).await },
    )
    .description(
        "Measure a question against items whose answers are already known, and report how \
         well it separates them",
    )
    .examples(examples())
    .mcp(read_only_remote("Measure a question"))
    .hint(HINT)
    .done()
}

/// What the numbers mean, and which of them to trust.
const HINT: &str = "\
Read `auc` first and treat it as the only portable number: it is the chance a \
true item scores above a false one, so 0.5 is a coin flip and below 0.5 means \
the question is answering backwards, which is a different problem from \
answering badly. Absolute probabilities move whenever the wording moves — \
naming the categories you care about inside the question can lift `auc` sharply \
while every individual score falls — so a threshold carried over from another \
phrasing is wrong. That is why `cuts` reports several rather than recommending \
one, and why --noul repeats: put several phrasings in one run and read them \
ranked. Compare `precision` against `base_rate`, because a gate no better than \
the base rate is not a gate, and check `positives` and `negatives` before \
quoting anything, because the number is only as good as the smaller side. For a \
choice question the gap between `accuracy` and `accuracy_when_confident` is \
what filtering on confidence would buy. `spread` and `undecided` need no labels \
at all: answers collapsing toward the middle mean the model had nothing to go \
on, and the first thing to check is whether the answer is present in the item \
text — a question about the state of a machine, or about what happens next, \
cannot be answered from the item however well it is worded. That test is \
one-sided, so a question that spreads widely can still be useless.";

/// Runs the command, mapping an SDK error to the CLI's stable codes.
async fn run(options: &Options) -> TypedResult<Evaluated> {
    match run::evaluate(options).await {
        Ok(result) => TypedResult::ok(result),
        Err(error) => TypedResult::error(code_for(&error), error.to_string()),
    }
}

/// Worked invocations, rendered into the skill file.
fn examples() -> Vec<Example> {
    vec![
        Example {
            command: "--labels-file graded.json --noul \"Is this ticket urgent?\"".to_owned(),
            description: Some("Measure a yes/no question against known answers".to_owned()),
        },
        Example {
            command: "--labels-file graded.json --noul \"Is this urgent?\" \
                      --noul \"Does this need attention today?\""
                .to_owned(),
            description: Some("Compare two wordings against the same items".to_owned()),
        },
    ]
}

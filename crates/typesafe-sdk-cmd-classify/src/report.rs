//! What a classification run reports.
//!
//! A failed item is a row, not an aborted run: one unclassifiable line in a
//! thousand should not cost the other nine hundred and ninety-nine. The
//! `uncertain` count is the one worth reading first, because a confident wrong
//! answer and an unconfident right one look identical in the labels alone.
//!
//! A batch reports what it cost in total. Per-item tokens are the caller's to
//! attribute; the number anyone actually quotes is the one for the whole run.

use schemars::JsonSchema;
use serde::Serialize;
use typesafe_sdk_cmd_kit::Usage;

/// One item and what came back for it.
#[derive(Serialize, JsonSchema)]
pub struct Item {
    /// The line that was classified.
    pub item: String,
    /// The answers, keyed by question name.
    pub answers: serde_json::Value,
    /// Whether any answer fell below the confidence threshold.
    pub uncertain: bool,
    /// What went wrong, when this item could not be classified.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// One item's result, and what answering it cost and involved.
pub struct Answered {
    /// The row to report.
    pub item: Item,
    /// Tokens spent on it.
    pub usage: Usage,
    /// The resolved model version that answered, if anything did.
    pub model: Option<String>,
}

/// Every item, in the order they were read.
#[derive(Serialize, JsonSchema)]
pub struct Classified {
    /// The resolved model version that answered, such as `jev-1.13.0`.
    ///
    /// This is the model the service actually used, not the alias that was
    /// asked for. It falls back to the configured default only when nothing
    /// was answered, because then no response named one.
    pub model: String,
    /// Every resolved version seen, when a run saw more than one.
    ///
    /// A long run can straddle a deployment. Reporting only the first would
    /// quietly mislabel everything after the roll.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub models: Option<Vec<String>>,
    /// One entry per input line.
    pub items: Vec<Item>,
    /// How many answers fell below the confidence threshold.
    pub uncertain: usize,
    /// How many items could not be classified at all.
    pub failed: usize,
    /// Tokens consumed by every item that was answered.
    pub usage: Usage,
}

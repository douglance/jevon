//! What a classification run reports.
//!
//! A failed item is a row, not an aborted run: one unclassifiable line in a
//! thousand should not cost the other nine hundred and ninety-nine. The
//! `uncertain` count is the one worth reading first, because a confident wrong
//! answer and an unconfident right one look identical in the labels alone.

use schemars::JsonSchema;
use serde::Serialize;

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

/// Every item, in the order they were read.
#[derive(Serialize, JsonSchema)]
pub struct Classified {
    /// The model that answered.
    pub model: String,
    /// One entry per input line.
    pub items: Vec<Item>,
    /// How many answers fell below the confidence threshold.
    pub uncertain: usize,
    /// How many items could not be classified at all.
    pub failed: usize,
}

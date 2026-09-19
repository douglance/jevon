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
    /// Tokens consumed by every item that was answered.
    pub usage: Usage,
}

impl Classified {
    /// Folds the per-item results, in order, into the reported shape.
    #[must_use]
    pub fn of(model: String, answered: Vec<(Item, Usage)>) -> Self {
        let mut usage = Usage::default();
        let items: Vec<Item> = answered
            .into_iter()
            .map(|(item, spent)| {
                usage.input_tokens = usage.input_tokens.saturating_add(spent.input_tokens);
                usage.output_tokens = usage.output_tokens.saturating_add(spent.output_tokens);
                item
            })
            .collect();
        Self {
            model,
            uncertain: items.iter().filter(|i| i.uncertain).count(),
            failed: items.iter().filter(|i| i.error.is_some()).count(),
            usage,
            items,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Classified, Item, Usage};

    fn item(uncertain: bool, error: Option<&str>) -> Item {
        Item {
            item: "x".to_owned(),
            answers: serde_json::Value::Null,
            uncertain,
            error: error.map(str::to_owned),
        }
    }

    fn spent(input: u64, output: u64) -> Usage {
        Usage {
            input_tokens: input,
            output_tokens: output,
        }
    }

    /// The reported total is the sum over the batch, not one item's and not
    /// the last one's — the distinction only shows up past two items.
    #[test]
    fn usage_totals_every_item() {
        let report = Classified::of(
            "jev-1".to_owned(),
            vec![
                (item(false, None), spent(10, 1)),
                (item(false, None), spent(200, 20)),
                (item(false, None), spent(3000, 300)),
            ],
        );
        assert_eq!(report.usage.input_tokens, 3210);
        assert_eq!(report.usage.output_tokens, 321);
    }

    /// An item that failed was never answered, so it adds nothing to the bill
    /// while still being counted as a row.
    #[test]
    fn a_failed_item_costs_nothing_and_still_counts() {
        let report = Classified::of(
            "jev-1".to_owned(),
            vec![
                (item(false, None), spent(7, 2)),
                (item(true, Some("boom")), Usage::default()),
            ],
        );
        assert_eq!(report.usage.input_tokens, 7);
        assert_eq!(report.failed, 1);
        assert_eq!(report.uncertain, 1);
        assert_eq!(report.items.len(), 2);
    }
}

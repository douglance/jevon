//! Turning per-item results into the run's report.
//!
//! Kept apart from the output types because this is where every count, total
//! and version is decided, and it is the part that has to be tested.

use indexmap::IndexMap;
use typesafe_sdk_cmd_kit::Usage;

use crate::report::{Answered, Classified, Item};

/// Adds one row's shaky questions to the running counts.
fn tally(counts: &mut IndexMap<String, usize>, shaky: Vec<String>) {
    for name in shaky {
        *counts.entry(name).or_default() += 1;
    }
}

/// Records a model version the first time it is seen, keeping the order they
/// appeared in so the first is the one a single-version run reports.
fn note(seen: &mut Vec<String>, model: Option<String>) {
    if let Some(model) = model.filter(|m| !seen.contains(m)) {
        seen.push(model);
    }
}

impl Classified {
    /// Folds the per-item results, in order, into the reported shape.
    #[must_use]
    pub fn of(configured: &str, answered: Vec<Answered>) -> Self {
        let mut usage = Usage::default();
        let mut seen: Vec<String> = Vec::new();
        let mut shaky: IndexMap<String, usize> = IndexMap::new();
        let mut items: Vec<Item> = Vec::with_capacity(answered.len());
        for row in answered {
            usage.input_tokens = usage.input_tokens.saturating_add(row.usage.input_tokens);
            usage.output_tokens = usage.output_tokens.saturating_add(row.usage.output_tokens);
            note(&mut seen, row.model);
            tally(&mut shaky, row.shaky);
            items.push(row.item);
        }
        Self {
            model: seen
                .first()
                .cloned()
                .unwrap_or_else(|| configured.to_owned()),
            models: (seen.len() > 1).then_some(seen),
            uncertain: items.iter().filter(|i| i.uncertain).count(),
            uncertain_by_question: shaky,
            failed: items.iter().filter(|i| i.error.is_some()).count(),
            usage,
            items,
        }
    }

    /// What the process should exit with.
    ///
    /// A failed item is a row, not an aborted run — but a run that answered
    /// nothing at all did not work, and exiting zero on it lets a broken key
    /// pass for success in any script or CI job.
    #[must_use]
    pub const fn exit_code(&self) -> Option<i32> {
        let answered = self.items.len().saturating_sub(self.failed);
        match (answered, self.failed) {
            (0, _) => Some(1),
            (_, 0) => None,
            _ => Some(2),
        }
    }
}

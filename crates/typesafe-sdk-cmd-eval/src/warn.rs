//! Saying what is wrong with a measurement, in the words a reader needs.
//!
//! A number that looks like a measurement and is not is worse than no number,
//! because it gets quoted. These are the two ways that happened here.

use crate::spread::degenerate;

/// Everything wrong with one question's measurement.
pub(crate) fn about(
    auc: Option<f64>,
    spread: Option<f64>,
    undecided: Option<f64>,
    items: usize,
) -> Vec<String> {
    let mut warnings = Vec::new();
    if items < crate::measure::ENOUGH {
        warnings.push(format!(
            "Only {items} items were comparable, so treat these numbers as a \
             direction rather than a measurement."
        ));
    }
    if degenerate(spread, undecided) {
        warnings.push(COLLAPSED.to_owned());
    }
    if auc.is_some_and(|value| value < 0.45) {
        warnings.push(BACKWARDS.to_owned());
    }
    warnings
}

/// The answers hedged. Said in terms of what to do about it.
const COLLAPSED: &str = "The answers collapsed toward the middle, which means the model had \
                         little to go on. Check whether the answer is present in the item text \
                         at all — a question about the state of a machine, or about what happens \
                         next, cannot be answered from the item however well it is worded. Note \
                         the converse does not hold: a question whose answers spread widely can \
                         still be useless.";

/// Below chance is a different problem from near chance, and worth naming.
const BACKWARDS: &str = "This question scored below chance, which means it is ordering the items \
                         backwards rather than failing to order them. Check that the question \
                         asks for what the labels actually mark.";

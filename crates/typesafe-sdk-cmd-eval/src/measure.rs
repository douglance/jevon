//! Turning answers plus known labels into one question's measurement.

use typesafe_sdk_answers::Answer;

use crate::report::{Cut, Measured};
use crate::score;
use crate::spread;
use crate::warn;

/// One item's answer next to the label it should have matched.
pub(crate) type Graded = (Option<Answer>, serde_json::Value);

/// A choice answer judged: right, confident, and how confident.
pub(crate) type Picked = (bool, bool, f64);

/// The smallest side of a labelled set worth quoting a number from.
///
/// Below this an AUC is a small sample rather than a measurement: 25 items once
/// reported 1.0 here and 0.673 on a rerun of the same question.
///
/// It is the *smaller* side that decides this. A set of 87 with only 18
/// positives is an 18-item measurement wearing an 87-item coat, and reading the
/// total is how that gets missed.
pub(crate) const ENOUGH: usize = 30;

/// Measures a yes/no question by how well it orders the labelled items.
pub(crate) fn noul(question: &str, scored: &[(f64, bool)]) -> Measured {
    let answers: Vec<f64> = scored.iter().map(|(p, _)| *p).collect();
    let positives: Vec<bool> = scored.iter().map(|(_, t)| *t).collect();
    let auc = score::auc(scored);
    let (sd, mid) = (spread::spread(&answers), spread::undecided(&answers));
    Measured {
        question: question.to_owned(),
        auc,
        base_rate: score::share(&positives),
        cuts: score::cuts(scored).iter().map(into_cut).collect(),
        accuracy: None,
        accuracy_when_confident: None,
        confident_share: None,
        spread: sd,
        undecided: mid,
        warnings: warn::about(auc, sd, mid, smaller_side(scored)),
    }
}

/// Measures a choice question by how often it picks the labelled answer, and
/// by how much a confidence filter would improve that.
pub(crate) fn choice(question: &str, picked: &[Picked]) -> Measured {
    let right: Vec<bool> = picked.iter().map(|(ok, _, _)| *ok).collect();
    let confident: Vec<bool> = picked
        .iter()
        .filter(|(_, c, _)| *c)
        .map(|(ok, _, _)| *ok)
        .collect();
    let cleared: Vec<bool> = picked.iter().map(|(_, c, _)| *c).collect();
    let confidences: Vec<f64> = picked.iter().map(|(_, _, v)| *v).collect();
    Measured {
        question: question.to_owned(),
        auc: None,
        base_rate: None,
        cuts: Vec::new(),
        accuracy: score::share(&right),
        accuracy_when_confident: score::share(&confident),
        confident_share: score::share(&cleared),
        spread: spread::spread(&confidences),
        undecided: None,
        warnings: warn::about(None, None, None, picked.len()),
    }
}

/// How many items are on the thinner side of the labels.
fn smaller_side(scored: &[(f64, bool)]) -> usize {
    let positives = scored.iter().filter(|(_, t)| *t).count();
    positives.min(scored.len() - positives)
}

/// Pairs a noul answer with its label, dropping what cannot be compared.
pub(crate) fn as_noul(answers: &[Graded]) -> Vec<(f64, bool)> {
    answers
        .iter()
        .filter_map(
            |(answer, label)| match (answer.as_ref()?, label.as_bool()?) {
                (Answer::Noul(a), label) => Some((a.noul, label)),
                _ => None,
            },
        )
        .collect()
}

/// Pairs a choice answer with its label as (right, confident, confidence).
pub(crate) fn as_choice(answers: &[Graded], threshold: f64) -> Vec<Picked> {
    answers
        .iter()
        .filter_map(
            |(answer, label)| match (answer.as_ref()?, label.as_str()?) {
                (Answer::Choice(a), label) => {
                    Some((a.choice == label, a.confidence >= threshold, a.confidence))
                }
                _ => None,
            },
        )
        .collect()
}

/// The scoring module's cut, as the reported one.
const fn into_cut(cut: &score::Cut) -> Cut {
    Cut {
        threshold: cut.threshold,
        flagged: cut.flagged,
        precision: cut.precision,
        recall: cut.recall,
    }
}

#[cfg(test)]
mod tests {
    use super::smaller_side;

    /// The count that decides whether a number is worth quoting is the thinner
    /// side, not the total. 87 items with 18 positives is an 18-item
    /// measurement, and reading the total is how that gets missed.
    #[test]
    fn the_smaller_side_is_what_counts() {
        let lopsided: Vec<(f64, bool)> = (0..87).map(|i| (0.5, i < 18)).collect();
        assert_eq!(smaller_side(&lopsided), 18);
    }

    #[test]
    fn an_even_split_counts_either_half() {
        let even: Vec<(f64, bool)> = (0..10).map(|i| (0.5, i < 5)).collect();
        assert_eq!(smaller_side(&even), 5);
    }

    /// One-sided labels measure nothing, and nothing is smaller than one side.
    #[test]
    fn one_sided_labels_have_no_thinner_side() {
        let all_true: Vec<(f64, bool)> = (0..40).map(|_| (0.5, true)).collect();
        assert_eq!(smaller_side(&all_true), 0);
    }
}

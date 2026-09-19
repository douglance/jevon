//! The two measurements that say whether a question works.
//!
//! For a yes/no question, the only number that survives rewording is the
//! ordering: AUC, the chance a true item scores above a false one. Absolute
//! probabilities shift whenever the wording shifts, so a threshold tuned on one
//! phrasing is wrong on the next, and is reported per-threshold rather than as
//! a single recommendation.
//!
//! For a choice question, accuracy is reported twice — over everything, and
//! over the answers the model was confident about. The gap between them is what
//! a confidence filter would buy.

/// The chance a positive scores above a negative, with ties counted as half.
///
/// 0.5 is a coin flip. Below 0.5 means the question is answering backwards,
/// which is a different problem from answering badly.
#[must_use]
pub(crate) fn auc(scored: &[(f64, bool)]) -> Option<f64> {
    let pos: Vec<f64> = scored.iter().filter(|(_, t)| *t).map(|(p, _)| *p).collect();
    let neg: Vec<f64> = scored
        .iter()
        .filter(|(_, t)| !*t)
        .map(|(p, _)| *p)
        .collect();
    if pos.is_empty() || neg.is_empty() {
        return None;
    }
    let wins: f64 = pos.iter().map(|a| beats(*a, &neg)).sum();
    #[expect(
        clippy::cast_precision_loss,
        reason = "counts this small are exact in f64"
    )]
    Some(wins / (pos.len() * neg.len()) as f64)
}

/// How many of `others` one score beats, counting a tie as half.
fn beats(score: f64, others: &[f64]) -> f64 {
    others
        .iter()
        .map(|other| {
            if score > *other {
                1.0
            } else if (score - *other).abs() < f64::EPSILON {
                0.5
            } else {
                0.0
            }
        })
        .sum()
}

/// How a threshold would behave if it were used as a gate.
pub(crate) struct Cut {
    /// The cut-off applied.
    pub(crate) threshold: f64,
    /// How many items it would flag.
    pub(crate) flagged: usize,
    /// What share of the flagged items really are positive.
    pub(crate) precision: f64,
    /// What share of all positives it would catch.
    pub(crate) recall: f64,
}

/// Walks the useful thresholds so the caller can pick one against their own
/// tolerance for a miss rather than being handed a number.
#[must_use]
pub(crate) fn cuts(scored: &[(f64, bool)]) -> Vec<Cut> {
    let total = scored.iter().filter(|(_, t)| *t).count();
    [0.9, 0.8, 0.7, 0.6, 0.5]
        .into_iter()
        .filter_map(|threshold| cut(scored, threshold, total))
        .collect()
}

/// One threshold, or nothing when it would flag no items at all.
fn cut(scored: &[(f64, bool)], threshold: f64, positives: usize) -> Option<Cut> {
    let flagged: Vec<bool> = scored
        .iter()
        .filter(|(p, _)| *p >= threshold)
        .map(|(_, t)| *t)
        .collect();
    if flagged.is_empty() || positives == 0 {
        return None;
    }
    let hits = flagged.iter().filter(|t| **t).count();
    #[expect(
        clippy::cast_precision_loss,
        reason = "counts this small are exact in f64"
    )]
    Some(Cut {
        threshold,
        flagged: flagged.len(),
        precision: hits as f64 / flagged.len() as f64,
        recall: hits as f64 / positives as f64,
    })
}

/// The share of `hits` that are true, or `None` when there is nothing to judge.
#[must_use]
pub(crate) fn share(hits: &[bool]) -> Option<f64> {
    #[expect(
        clippy::cast_precision_loss,
        reason = "counts this small are exact in f64"
    )]
    (!hits.is_empty()).then(|| hits.iter().filter(|h| **h).count() as f64 / hits.len() as f64)
}

#[cfg(test)]
mod tests {
    use super::{auc, cuts, share};

    /// Every expected value here is counted by hand from the definition — the
    /// chance a positive outranks a negative — so the test does not agree with
    /// the code merely because both came from the same idea.
    #[test]
    fn auc_counts_the_pairs_a_positive_wins() {
        // One pair, positive on top: it wins, so 1 of 1.
        assert_eq!(auc(&[(0.9, true), (0.1, false)]), Some(1.0));
        // The same pair the wrong way up: it loses every time.
        assert_eq!(auc(&[(0.1, true), (0.9, false)]), Some(0.0));
        // Four pairs: 0.8 beats both, 0.3 beats only 0.1. Three of four.
        assert_eq!(
            auc(&[(0.8, true), (0.3, true), (0.5, false), (0.1, false)]),
            Some(0.75)
        );
    }

    /// A tie is half a win, which is what makes a question that answers the
    /// same value for everything score exactly chance rather than zero.
    #[test]
    fn a_tie_counts_as_half() {
        assert_eq!(auc(&[(0.5, true), (0.5, false)]), Some(0.5));
        assert_eq!(
            auc(&[(0.4, true), (0.4, true), (0.4, false), (0.4, false)]),
            Some(0.5)
        );
    }

    /// With nothing to compare against there is no ordering to report, and
    /// reporting 0.5 would claim a measurement that was never made.
    #[test]
    fn one_sided_labels_measure_nothing() {
        assert_eq!(auc(&[(0.9, true), (0.8, true)]), None);
        assert_eq!(auc(&[]), None);
    }

    /// Precision is of the flagged rows and recall is of all the positives;
    /// swapping the two denominators is the easiest mistake here to make.
    #[test]
    fn a_cut_divides_precision_and_recall_differently() {
        // At 0.5: flags 0.9(t), 0.6(f), 0.5(t) — 2 right of 3 flagged,
        // and 2 of the 4 positives overall.
        let scored = [
            (0.9, true),
            (0.6, false),
            (0.5, true),
            (0.4, true),
            (0.1, true),
        ];
        let cut = cuts(&scored)
            .into_iter()
            .find(|c| (c.threshold - 0.5).abs() < f64::EPSILON)
            .expect("a cut at 0.5 flags three rows");
        assert_eq!(cut.flagged, 3);
        assert!((cut.precision - 2.0 / 3.0).abs() < 1e-9, "2 of 3 flagged");
        assert!((cut.recall - 0.5).abs() < 1e-9, "2 of 4 positives");
    }

    /// A threshold nothing reaches is left out rather than reported as a
    /// perfect gate that flags nothing.
    #[test]
    fn a_cut_nothing_reaches_is_not_reported() {
        let thresholds: Vec<f64> = cuts(&[(0.2, true), (0.1, false)])
            .iter()
            .map(|c| c.threshold)
            .collect();
        assert!(thresholds.is_empty(), "got {thresholds:?}");
    }

    #[test]
    fn share_is_the_proportion_that_are_true() {
        assert_eq!(share(&[true, false, false, false]), Some(0.25));
        assert_eq!(share(&[]), None);
    }
}

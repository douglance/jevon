//! Whether a question got any purchase, judged without knowing the answers.
//!
//! `eval` needs labels and most callers will not have any. But the worst
//! failure mode shows up in the answers alone: when the model has nothing to go
//! on it hedges, and every probability collapses toward the middle. Measured
//! across seven real runs, the three questions whose answers sat in the middle
//! band were all at or below chance, including one that produced a clean-looking
//! "86% yes" from a set whose every answer was between 0.4 and 0.8.
//!
//! **This is a one-sided test and the warning has to say so.** A question that
//! spreads well is not thereby a good question — "will this shell command fail"
//! spread as widely as the best question measured here and still scored 0.665
//! against ground truth. Collapse proves failure; spread proves nothing.

/// How far the answers sit from the middle, as a standard deviation.
#[must_use]
pub(crate) fn spread(answers: &[f64]) -> Option<f64> {
    let mean = mean(answers)?;
    let variance = mean_of(
        &answers
            .iter()
            .map(|a| (a - mean).powi(2))
            .collect::<Vec<_>>(),
    )?;
    Some(variance.sqrt())
}

/// The share of answers that committed to neither side.
#[must_use]
pub(crate) fn undecided(answers: &[f64]) -> Option<f64> {
    let inside: Vec<f64> = answers
        .iter()
        .map(|a| f64::from(u8::from((0.35..0.65).contains(a))))
        .collect();
    mean_of(&inside)
}

/// Whether the answers collapsed toward the middle, which means the question
/// could not be answered from the items at all.
///
/// The thresholds come from the seven measured runs: everything at or below
/// chance had a spread under 0.17 and more than half its answers in the middle
/// band, and nothing that scored well came close to either.
#[must_use]
pub(crate) fn degenerate(spread: Option<f64>, undecided: Option<f64>) -> bool {
    matches!((spread, undecided), (Some(s), Some(u)) if s < 0.18 && u > 0.5)
}

fn mean(values: &[f64]) -> Option<f64> {
    mean_of(values)
}

#[expect(
    clippy::cast_precision_loss,
    reason = "a batch large enough to lose precision here cannot be paid for"
)]
fn mean_of(values: &[f64]) -> Option<f64> {
    (!values.is_empty()).then(|| values.iter().sum::<f64>() / values.len() as f64)
}

#[cfg(test)]
#[expect(
    clippy::unwrap_used,
    reason = "a test that cannot fail loudly is not a test"
)]
mod tests {
    use super::{degenerate, spread, undecided};

    /// Hand-computable: mean 0.5, deviations ±0.4, so the standard deviation
    /// is 0.4 exactly.
    #[test]
    fn spread_is_the_standard_deviation() {
        let sd = spread(&[0.9, 0.1, 0.9, 0.1]).unwrap();
        assert!((sd - 0.4).abs() < 1e-12, "got {sd}");
        assert_eq!(spread(&[0.5, 0.5, 0.5]), Some(0.0));
        assert_eq!(spread(&[]), None);
    }

    /// The band is 0.35 up to but not including 0.65, so its edges decide
    /// exactly one way each.
    #[test]
    fn undecided_counts_the_middle_band_only() {
        assert_eq!(undecided(&[0.5, 0.9]), Some(0.5));
        assert_eq!(undecided(&[0.34, 0.35, 0.64, 0.65]), Some(0.5));
        assert_eq!(undecided(&[0.99, 0.01]), Some(0.0));
    }

    /// The real numbers from the runs this was derived from: the bottom three
    /// of that table must trip, and the questions that measured well must not.
    #[test]
    fn it_trips_on_the_runs_that_scored_at_chance() {
        assert!(
            degenerate(Some(0.096), Some(0.62)),
            "will the user push back"
        );
        assert!(
            degenerate(Some(0.168), Some(0.56)),
            "will this session run long"
        );
        assert!(degenerate(Some(0.079), Some(0.80)), "was this preventable");
        assert!(
            !degenerate(Some(0.285), Some(0.32)),
            "hard to undo, AUC 0.997"
        );
        assert!(
            !degenerate(Some(0.324), Some(0.15)),
            "destroy work, AUC 0.872"
        );
    }

    /// The limit of the test, asserted so nobody widens the claim: this run
    /// scored 0.665 against ground truth and spreads like a good one. Collapse
    /// proves failure; spread proves nothing.
    #[test]
    fn it_does_not_catch_a_bad_question_that_spreads() {
        assert!(
            !degenerate(Some(0.253), Some(0.28)),
            "will this command fail"
        );
        assert!(
            !degenerate(Some(0.258), Some(0.29)),
            "claims success, AUC 0.485"
        );
    }
}

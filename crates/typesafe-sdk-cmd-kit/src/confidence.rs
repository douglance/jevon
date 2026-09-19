//! Whether an answer is confident enough for a caller to act on.
//!
//! A threshold is not portable. Rewording a question moves every probability it
//! produces — one rewrite here lifted a question's ordering from 0.87 to 0.997
//! while every individual score fell — so a number tuned against one phrasing
//! is wrong against the next. The threshold belongs to the question, not to the
//! tool, which is why this only answers the comparison and never picks a value.

use typesafe_sdk_answers::Answer;

/// Whether an answer is less confident than the caller will accept.
///
/// A noul has no confidence of its own: its probability *is* the answer, and a
/// value near the middle is the uncertain case. The threshold becomes a band
/// either side of 0.5, so 0 accepts everything and 1 accepts only a decided
/// yes or no — the same direction of travel as a choice's confidence.
#[must_use]
pub fn below(answer: &Answer, threshold: f64) -> bool {
    match answer {
        Answer::Choice(a) => a.confidence < threshold,
        Answer::Score(a) => a.confidence < threshold,
        Answer::Noul(a) => (a.noul - 0.5).abs() < threshold / 2.0,
    }
}

#[cfg(test)]
mod tests {
    use super::below;
    use typesafe_sdk_answers::{Answer, ChoiceAnswer, NoulAnswer};

    fn choice(confidence: f64) -> Answer {
        Answer::Choice(ChoiceAnswer {
            choice: "a".to_owned(),
            confidence,
            probabilities: indexmap::IndexMap::new(),
        })
    }

    #[test]
    fn a_choice_below_the_threshold_is_uncertain() {
        assert!(below(&choice(0.37), 0.5));
        assert!(!below(&choice(0.99), 0.5));
    }

    #[test]
    fn the_threshold_is_exclusive_at_the_boundary() {
        assert!(!below(&choice(0.5), 0.5));
    }

    /// A noul has no confidence of its own; a probability near the middle is
    /// the uncertain case, and one near either end is a decided answer.
    #[test]
    fn a_noul_is_judged_by_distance_from_the_middle() {
        assert!(below(&Answer::Noul(NoulAnswer { noul: 0.5 }), 0.5));
        assert!(below(&Answer::Noul(NoulAnswer { noul: 0.6 }), 0.5));
        assert!(!below(&Answer::Noul(NoulAnswer { noul: 0.9 }), 0.5));
        assert!(!below(&Answer::Noul(NoulAnswer { noul: 0.05 }), 0.5));
    }

    #[test]
    fn a_zero_threshold_accepts_everything() {
        assert!(!below(&choice(0.0), 0.0));
        assert!(!below(&Answer::Noul(NoulAnswer { noul: 0.5 }), 0.0));
    }
}

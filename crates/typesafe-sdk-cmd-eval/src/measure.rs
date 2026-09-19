//! Turning answers plus known labels into the report.

use typesafe_sdk_answers::Answer;

use crate::labels::Labelled;
use crate::report::{Cut, Evaluated};
use crate::score;

/// One answered item, paired with what it should have been.
pub(crate) struct Judged {
    /// The answer that came back.
    pub(crate) answer: Option<Answer>,
    /// The label it should have matched.
    pub(crate) label: serde_json::Value,
}

/// Pairs each answer with its label, dropping nothing so `failed` stays honest.
pub(crate) fn judge(rows: Vec<Labelled>, answers: Vec<Option<Answer>>) -> Vec<Judged> {
    rows.into_iter()
        .zip(answers)
        .map(|(row, answer)| Judged {
            answer,
            label: row.label,
        })
        .collect()
}

/// Measures a yes/no question by how well it orders the labelled items.
pub(crate) fn noul(
    judged: &[Judged],
    model: String,
    usage: typesafe_sdk_cmd_kit::Usage,
) -> Evaluated {
    let scored: Vec<(f64, bool)> = judged
        .iter()
        .filter_map(|j| match (j.answer.as_ref()?, j.label.as_bool()?) {
            (Answer::Noul(a), label) => Some((a.noul, label)),
            _ => None,
        })
        .collect();
    let positives: Vec<bool> = scored.iter().map(|(_, t)| *t).collect();
    Evaluated {
        model,
        items: judged.len(),
        failed: judged.iter().filter(|j| j.answer.is_none()).count(),
        kind: "noul".to_owned(),
        auc: score::auc(&scored),
        base_rate: score::share(&positives),
        cuts: score::cuts(&scored).iter().map(into_cut).collect(),
        accuracy: None,
        accuracy_when_confident: None,
        confident_share: None,
        usage,
    }
}

/// Measures a choice question by how often it picks the labelled answer, and
/// by how much a confidence filter would improve that.
pub(crate) fn choice(
    judged: &[Judged],
    threshold: f64,
    model: String,
    usage: typesafe_sdk_cmd_kit::Usage,
) -> Evaluated {
    let picked: Vec<(bool, bool)> = judged
        .iter()
        .filter_map(|j| match (j.answer.as_ref()?, j.label.as_str()?) {
            (Answer::Choice(a), label) => Some((a.choice == label, a.confidence >= threshold)),
            _ => None,
        })
        .collect();
    let right: Vec<bool> = picked.iter().map(|(ok, _)| *ok).collect();
    let confident: Vec<bool> = picked
        .iter()
        .filter(|(_, c)| *c)
        .map(|(ok, _)| *ok)
        .collect();
    let cleared: Vec<bool> = picked.iter().map(|(_, c)| *c).collect();
    Evaluated {
        model,
        items: judged.len(),
        failed: judged.iter().filter(|j| j.answer.is_none()).count(),
        kind: "choice".to_owned(),
        auc: None,
        base_rate: None,
        cuts: Vec::new(),
        accuracy: score::share(&right),
        accuracy_when_confident: score::share(&confident),
        confident_share: score::share(&cleared),
        usage,
    }
}

/// The scoring crate's cut, as the reported one.
const fn into_cut(cut: &score::Cut) -> Cut {
    Cut {
        threshold: cut.threshold,
        flagged: cut.flagged,
        precision: cut.precision,
        recall: cut.recall,
    }
}

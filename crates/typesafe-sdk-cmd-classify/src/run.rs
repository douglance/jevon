//! Running one question set over many items.

use futures::StreamExt as _;
use typesafe_sdk_answers::Answer;
use typesafe_sdk_answers::SystemOneResponse;
use typesafe_sdk_client::{Client, SystemOneRequest};
use typesafe_sdk_cmd_kit::{Usage, client, items as read_items, lines};

use crate::report::Answered;
use typesafe_sdk_error::Result;
use typesafe_sdk_questions::Questions;

use crate::Options;
use crate::asking::question_set;
use crate::report::{Classified, Item};

/// Classifies every line of standard input.
///
/// # Errors
/// Returns [`Error::Invalid`] when no questions were given or stdin held no
/// items, and a client error when one cannot be built.
pub(crate) async fn classify(options: &Options) -> Result<Classified> {
    let asked = question_set(options)?;
    let items = match options.items_file.as_deref() {
        Some(path) => read_items(path)?,
        None => lines()?,
    };
    let client = client()?;
    let threshold = options.min_confidence;

    let answered: Vec<Answered> = futures::stream::iter(items.into_iter().map(|item| {
        let client = &client;
        let job = Job {
            asked: asked.clone(),
            model: options.model.clone(),
            threshold,
        };
        async move { one(client, item, job).await }
    }))
    .buffered(options.concurrency.max(1) as usize)
    .collect()
    .await;

    Ok(Classified::of(&client.config().default_model, answered))
}

/// What every item in a run is judged against.
#[derive(Clone)]
struct Job {
    asked: Questions,
    model: Option<String>,
    threshold: f64,
}

/// Classifies one item, reporting a failure rather than abandoning the run.
///
/// A failed item spent nothing we can account for, so it contributes no tokens
/// to the run's total.
async fn one(client: &Client, item: String, job: Job) -> Answered {
    let Job {
        asked,
        model,
        threshold,
    } = job;
    let mut request = SystemOneRequest::new(item.as_str(), asked);
    if let Some(model) = model {
        request = request.model(model);
    }
    match client.system_one(request).await {
        Ok(response) => answered(item, &response, threshold),
        Err(error) => failed(item, &error.to_string()),
    }
}

/// One item the model answered, what it cost, and which model version answered.
fn answered(item: String, response: &SystemOneResponse, threshold: f64) -> Answered {
    Answered {
        item: Item {
            item,
            uncertain: response.answers.values().any(|a| below(a, threshold)),
            answers: serde_json::to_value(&response.answers).unwrap_or(serde_json::Value::Null),
            error: None,
        },
        usage: Usage::from(&response.usage),
        model: Some(response.model.clone()),
    }
}

/// One item the run could not answer, reported as a row rather than an abort.
///
/// Nothing answered it, so it names no model and spent nothing we can account
/// for.
fn failed(item: String, error: &str) -> Answered {
    Answered {
        item: Item {
            item,
            answers: serde_json::Value::Null,
            uncertain: true,
            error: Some(error.to_owned()),
        },
        usage: Usage::default(),
        model: None,
    }
}

/// Whether an answer is less confident than the caller will accept.
///
/// A noul has no confidence of its own: its probability *is* the answer, and a
/// value near the middle is the uncertain case. The threshold becomes a band
/// either side of 0.5, so 0 accepts everything and 1 accepts only a decided
/// yes or no — the same direction of travel as a choice's confidence.
fn below(answer: &Answer, threshold: f64) -> bool {
    match answer {
        Answer::Choice(a) => a.confidence < threshold,
        Answer::Score(a) => a.confidence < threshold,
        Answer::Noul(a) => (a.noul - 0.5).abs() < threshold / 2.0,
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    reason = "a test that cannot fail loudly is not a test"
)]
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

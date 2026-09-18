//! Running one question set over many items.

use futures::StreamExt as _;
use typesafe_sdk_answers::Answer;
use typesafe_sdk_client::{Client, SystemOneRequest};
use typesafe_sdk_cmd_kit::{client, lines, questions as parse_questions};
use typesafe_sdk_error::{Error, Result};
use typesafe_sdk_questions::{Questions, choice_of, noul, questions, score};

use crate::Options;
use crate::report::{Classified, Item};

/// The name a shorthand question is answered under.
const SHORTHAND: &str = "answer";

/// Classifies every line of standard input.
///
/// # Errors
/// Returns [`Error::Invalid`] when no questions were given or stdin held no
/// items, and a client error when one cannot be built.
pub(crate) async fn classify(options: &Options) -> Result<Classified> {
    let asked = question_set(options)?;
    let items = lines()?;
    let client = client()?;
    let threshold = options.min_confidence;

    let answered: Vec<Item> = futures::stream::iter(items.into_iter().map(|item| {
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

    Ok(summarise(&client, answered))
}

/// Folds the per-item results into the reported shape.
fn summarise(client: &Client, items: Vec<Item>) -> Classified {
    Classified {
        model: client.config().default_model.clone(),
        uncertain: items.iter().filter(|i| i.uncertain).count(),
        failed: items.iter().filter(|i| i.error.is_some()).count(),
        items,
    }
}

/// What every item in a run is judged against.
#[derive(Clone)]
struct Job {
    asked: Questions,
    model: Option<String>,
    threshold: f64,
}

/// Classifies one item, reporting a failure rather than abandoning the run.
async fn one(client: &Client, item: String, job: Job) -> Item {
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
        Ok(response) => Item {
            item,
            uncertain: response.answers.values().any(|a| below(a, threshold)),
            answers: serde_json::to_value(&response.answers).unwrap_or(serde_json::Value::Null),
            error: None,
        },
        Err(error) => Item {
            item,
            answers: serde_json::Value::Null,
            uncertain: true,
            error: Some(error.to_string()),
        },
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

/// The questions to ask, from a file, inline JSON, or the shorthands.
fn question_set(options: &Options) -> Result<Questions> {
    if let Some(set) = parse_questions(
        options.questions.as_deref(),
        options.questions_file.as_deref(),
    )? {
        return Ok(set);
    }
    shorthand(options).ok_or_else(|| {
        Error::Invalid(
            "No questions were given. Pass --questions, --questions-file, or one of \
             --choice, --score or --noul."
                .to_owned(),
        )
    })
}

fn shorthand(options: &Options) -> Option<Questions> {
    let instructions = options.noul.as_deref().unwrap_or("");
    if !options.choice.is_empty() {
        return Some(questions([(
            SHORTHAND,
            choice_of(instructions, options.choice.clone()),
        )]));
    }
    if !options.score.is_empty() {
        return Some(questions([(
            SHORTHAND,
            score(instructions, options.score.clone()),
        )]));
    }
    options
        .noul
        .as_deref()
        .map(|text| questions([(SHORTHAND, noul(text))]))
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

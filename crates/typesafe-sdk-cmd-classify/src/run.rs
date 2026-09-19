//! Running one question set over many items.

use futures::StreamExt as _;
use typesafe_sdk_answers::SystemOneResponse;
use typesafe_sdk_client::{Client, SystemOneRequest};
use typesafe_sdk_cmd_kit::{Usage, below, client, read, resolve};

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
    let items = read(resolve(
        &options.items,
        options.items_file.as_deref(),
        options.stdin,
    )?)?;
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
    let shaky: Vec<String> = response
        .answers
        .iter()
        .filter(|(_, answer)| below(answer, threshold))
        .map(|(name, _)| name.clone())
        .collect();
    Answered {
        item: Item {
            item,
            uncertain: !shaky.is_empty(),
            answers: serde_json::to_value(&response.answers).unwrap_or(serde_json::Value::Null),
            error: None,
        },
        usage: Usage::from(&response.usage),
        model: Some(response.model.clone()),
        shaky,
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
        shaky: Vec::new(),
    }
}

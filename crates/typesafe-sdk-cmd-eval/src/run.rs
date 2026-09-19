//! Asking one question of every labelled item.

use futures::StreamExt as _;
use typesafe_sdk_answers::Answer;
use typesafe_sdk_client::{Client, SystemOneRequest};
use typesafe_sdk_cmd_kit::{Usage, client};
use typesafe_sdk_error::{Error, Result};
use typesafe_sdk_questions::{Questions, choice_of, noul, questions};

use crate::Options;
use crate::labels::{Labelled, read};
use crate::measure::{Judged, choice, judge, noul as measure_noul};
use crate::report::Evaluated;

/// The name the single question is answered under.
const ASKED: &str = "answer";

/// One item's answer, what it cost, and which model version answered.
type Reply = (Option<Answer>, Usage, Option<String>);

/// Runs the eval and reports what it measured.
///
/// # Errors
/// Returns [`Error::Invalid`] when no question was given or the labels cannot
/// be read, and a client error when one cannot be built.
pub(crate) async fn evaluate(options: &Options) -> Result<Evaluated> {
    let asked = question(options)?;
    let rows = read(&options.labels_file)?;
    let client = client()?;
    let (answers, usage, model) = ask_all(&client, &rows, &asked, options).await;
    let judged = judge(rows, answers);
    let model = model.unwrap_or_else(|| client.config().default_model.clone());
    Ok(report(&judged, options, model, usage))
}

/// Which measurement the question kind calls for.
fn report(judged: &[Judged], options: &Options, model: String, usage: Usage) -> Evaluated {
    if options.choice.is_empty() {
        measure_noul(judged, model, usage)
    } else {
        choice(judged, options.min_confidence, model, usage)
    }
}

/// Asks every item, keeping order and letting a failed item be a hole rather
/// than an abort — an eval over 400 items should not die on one of them.
async fn ask_all(
    client: &Client,
    rows: &[Labelled],
    asked: &Questions,
    options: &Options,
) -> (Vec<Option<Answer>>, Usage, Option<String>) {
    // Collecting first is load-bearing: a stream built straight off `rows.iter()`
    // borrows for a lifetime the async boundary cannot name, and the closure
    // stops satisfying the higher-ranked bound `CommandDef::typed` requires.
    #[expect(
        clippy::needless_collect,
        reason = "the borrow, not the allocation, is what this removes"
    )]
    let texts: Vec<String> = rows.iter().map(|row| row.item.clone()).collect();
    let answered: Vec<Reply> = futures::stream::iter(texts.into_iter().map(|item| {
        let asked = asked.clone();
        let model = options.model.clone();
        async move { one(client, item, asked, model).await }
    }))
    .buffered(options.concurrency.max(1) as usize)
    .collect()
    .await;
    let mut usage = Usage::default();
    let mut model = None;
    let mut answers = Vec::with_capacity(answered.len());
    for (answer, spent, seen) in answered {
        usage.input_tokens = usage.input_tokens.saturating_add(spent.input_tokens);
        usage.output_tokens = usage.output_tokens.saturating_add(spent.output_tokens);
        model = model.or(seen);
        answers.push(answer);
    }
    (answers, usage, model)
}

/// One item, answered or not.
async fn one(client: &Client, item: String, asked: Questions, model: Option<String>) -> Reply {
    let mut request = SystemOneRequest::new(item.as_str(), asked);
    if let Some(model) = model {
        request = request.model(model);
    }
    (client.system_one(request).await).map_or((None, Usage::default(), None), |response| {
        (
            response.answers.get(ASKED).cloned(),
            Usage::from(&response.usage),
            Some(response.model),
        )
    })
}

/// The one question being measured.
fn question(options: &Options) -> Result<Questions> {
    let instructions = options.noul.as_deref().unwrap_or_default();
    if !options.choice.is_empty() {
        return Ok(questions([(
            ASKED,
            choice_of(instructions, options.choice.clone()),
        )]));
    }
    options.noul.as_deref().map_or_else(
        || {
            Err(Error::Invalid(
                "No question was given. Pass --noul, or --noul with --choice.".to_owned(),
            ))
        },
        |text| Ok(questions([(ASKED, noul(text))])),
    )
}

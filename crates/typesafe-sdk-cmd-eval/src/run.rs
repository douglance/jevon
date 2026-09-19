//! Asking each question of every labelled item.

use futures::StreamExt as _;
use typesafe_sdk_answers::Answer;
use typesafe_sdk_client::{Client, SystemOneRequest};
use typesafe_sdk_cmd_kit::{Usage, client};
use typesafe_sdk_error::{Error, Result};
use typesafe_sdk_questions::{Questions, choice_of, noul, questions};

use crate::Options;
use crate::labels::{Labelled, read};
use crate::measure::{Graded, as_choice, as_noul, choice, noul as measure_noul};
use crate::report::{Evaluated, Measured};

/// The name the question is answered under.
const ASKED: &str = "answer";

/// One item's answer, what it cost, and which model version answered.
type Reply = (Option<Answer>, Usage, Option<String>);

/// Runs the eval and reports what it measured, best question first.
///
/// # Errors
/// Returns [`Error::Invalid`] when no question was given or the labels cannot
/// be read, and a client error when one cannot be built.
pub(crate) async fn evaluate(options: &Options) -> Result<Evaluated> {
    if options.noul.is_empty() {
        return Err(Error::Invalid(
            "No question was given. Pass --noul, and repeat it to compare wordings.".to_owned(),
        ));
    }
    let rows = read(&options.labels_file)?;
    let client = client()?;
    let Run {
        mut measured,
        usage,
        model,
        failed,
    } = each(&client, &rows, options).await;
    measured.sort_by(|a, b| rank(b).total_cmp(&rank(a)));
    Ok(Evaluated {
        model: model.unwrap_or_else(|| client.config().default_model.clone()),
        items: rows.len(),
        failed,
        positives: sided(&rows, true),
        negatives: sided(&rows, false),
        questions: measured,
        usage,
    })
}

/// What asking every question of every item produced.
struct Run {
    measured: Vec<Measured>,
    usage: Usage,
    model: Option<String>,
    failed: usize,
}

/// Measures each question in turn, over the same items.
async fn each(client: &Client, rows: &[Labelled], options: &Options) -> Run {
    let mut run = Run {
        measured: Vec::with_capacity(options.noul.len()),
        usage: Usage::default(),
        model: None,
        failed: 0,
    };
    for question in &options.noul {
        let (answers, spent, seen) = ask_all(client, rows, question, options).await;
        run.usage.input_tokens = run.usage.input_tokens.saturating_add(spent.input_tokens);
        run.usage.output_tokens = run.usage.output_tokens.saturating_add(spent.output_tokens);
        run.model = run.model.or(seen);
        run.failed += answers.iter().filter(|(a, _)| a.is_none()).count();
        run.measured.push(measure(question, &answers, options));
    }
    run
}

/// How many rows carry one side of a yes/no label.
fn sided(rows: &[Labelled], want: bool) -> usize {
    rows.iter()
        .filter(|r| r.label.as_bool() == Some(want))
        .count()
}

/// What a question is ranked by. A choice has no AUC, so accuracy stands in.
fn rank(measured: &Measured) -> f64 {
    measured.auc.or(measured.accuracy).unwrap_or(0.0)
}

/// Which measurement the question kind calls for.
fn measure(question: &str, answers: &[Graded], options: &Options) -> Measured {
    if options.choice.is_empty() {
        measure_noul(question, &as_noul(answers))
    } else {
        choice(question, &as_choice(answers, options.min_confidence))
    }
}

/// Asks every item, keeping order and letting a failed item be a hole rather
/// than an abort — an eval over 400 items should not die on one of them.
async fn ask_all(
    client: &Client,
    rows: &[Labelled],
    question: &str,
    options: &Options,
) -> (
    Vec<(Option<Answer>, serde_json::Value)>,
    Usage,
    Option<String>,
) {
    let asked = question_set(question, options);
    // Collecting first is load-bearing: a stream built straight off `rows.iter()`
    // borrows for a lifetime the async boundary cannot name, and the closure
    // stops satisfying the higher-ranked bound `CommandDef::typed` requires.
    #[expect(
        clippy::needless_collect,
        reason = "the borrow, not the allocation, is what this removes"
    )]
    let texts: Vec<String> = rows.iter().map(|row| row.item.clone()).collect();
    let replies: Vec<Reply> = futures::stream::iter(texts.into_iter().map(|item| {
        let asked = asked.clone();
        let model = options.model.clone();
        async move { one(client, item, asked, model).await }
    }))
    .buffered(options.concurrency.max(1) as usize)
    .collect()
    .await;

    let mut usage = Usage::default();
    let mut model = None;
    let mut answers = Vec::with_capacity(replies.len());
    for ((answer, spent, seen), row) in replies.into_iter().zip(rows) {
        usage.input_tokens = usage.input_tokens.saturating_add(spent.input_tokens);
        usage.output_tokens = usage.output_tokens.saturating_add(spent.output_tokens);
        model = model.or(seen);
        answers.push((answer, row.label.clone()));
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

/// The one question being measured, in the kind the labels call for.
fn question_set(question: &str, options: &Options) -> Questions {
    if options.choice.is_empty() {
        questions([(ASKED, noul(question))])
    } else {
        questions([(ASKED, choice_of(question, options.choice.clone()))])
    }
}

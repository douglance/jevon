//! Turning flags into a question set.

use typesafe_sdk_error::{Error, Result};
use typesafe_sdk_questions::{Questions, choice_of, noul, questions, score};

use crate::Options;

/// The name a shorthand question is answered under.
const SHORTHAND: &str = "answer";

/// Builds the question set the flags describe.
///
/// # Errors
/// Returns [`Error::Invalid`] when no question was given, or when `--questions`
/// is not a JSON object of questions.
pub(crate) fn questions_from(options: &Options) -> Result<Questions> {
    if let Some(raw) = options.questions.as_deref() {
        return from_json(raw);
    }
    shorthand(options).ok_or_else(|| {
        Error::Invalid(
            "No questions were given. Pass --questions with a JSON object, or one of \
             --choice, --score or --noul."
                .to_owned(),
        )
    })
}

/// The single-question shorthands, in the order they are checked.
fn shorthand(options: &Options) -> Option<Questions> {
    let instructions = options.noul.as_deref().unwrap_or("");
    if !options.choice.is_empty() {
        let labels = options.choice.clone();
        return Some(questions([(SHORTHAND, choice_of(instructions, labels))]));
    }
    if !options.score.is_empty() {
        let rubric = options.score.clone();
        return Some(questions([(SHORTHAND, score(instructions, rubric))]));
    }
    options
        .noul
        .as_deref()
        .map(|text| questions([(SHORTHAND, noul(text))]))
}

/// Parses a `--questions` payload.
fn from_json(raw: &str) -> Result<Questions> {
    serde_json::from_str(raw).map_err(|error| {
        Error::Invalid(format!(
            "`--questions` must be a JSON object of questions keyed by answer name: {error}."
        ))
    })
}

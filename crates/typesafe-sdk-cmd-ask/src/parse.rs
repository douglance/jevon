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
    if let Some(set) = typesafe_sdk_cmd_kit::questions(
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

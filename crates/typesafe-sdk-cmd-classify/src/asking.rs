//! Choosing what to ask every item.
//!
//! The shorthands exist because the common case is one question, and writing
//! it as JSON to ask it once is friction with no payoff. They are checked in a
//! fixed order so a caller who passes two of them gets a predictable set
//! rather than whichever the map happened to yield first.

use typesafe_sdk_cmd_kit::questions as parse_questions;
use typesafe_sdk_error::{Error, Result};
use typesafe_sdk_questions::{Questions, choice_of, noul, questions, score};

use crate::Options;

/// The name a shorthand question is answered under.
const SHORTHAND: &str = "answer";

/// The questions to ask, from a file, inline JSON, or the shorthands.
pub(crate) fn question_set(options: &Options) -> Result<Questions> {
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

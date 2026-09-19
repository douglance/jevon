//! Getting state and questions in from somewhere other than argv.
//!
//! Both of these exist because of what a real classification run looked like:
//! a question set large enough that shell quoting kept mangling it, and states
//! too long to pass as arguments. A CLI that can only be driven by argv forces
//! the caller into another language to use it at all.

use std::io::Read as _;

use typesafe_sdk_error::{Error, Result};
use typesafe_sdk_questions::Questions;

/// The argument that means "read this from standard input".
pub const STDIN: &str = "-";

/// Reads `value`, or standard input when it is `-`.
///
/// # Errors
/// Returns [`Error::Invalid`] when standard input cannot be read or is empty.
pub fn text(value: &str) -> Result<String> {
    if value != STDIN {
        return Ok(value.to_owned());
    }
    let mut buffer = String::new();
    std::io::stdin()
        .read_to_string(&mut buffer)
        .map_err(|error| Error::Invalid(format!("could not read standard input: {error}")))?;
    if buffer.trim().is_empty() {
        return Err(Error::Invalid(
            "standard input was empty; `-` means read the text from stdin".to_owned(),
        ));
    }
    Ok(buffer)
}

/// Parses a question set from inline JSON or a file.
///
/// # Errors
/// Returns [`Error::Invalid`] when the file cannot be read or the JSON is not
/// an object of questions.
pub fn questions(inline: Option<&str>, path: Option<&str>) -> Result<Option<Questions>> {
    let Some(found) = source(inline, path)? else {
        return Ok(None);
    };
    parse(&found.raw, found.flag).map(Some)
}

/// Question text and the flag it arrived on.
struct Source {
    raw: String,
    flag: &'static str,
}

/// Where the questions came from, or nothing if neither flag was given.
fn source(inline: Option<&str>, path: Option<&str>) -> Result<Option<Source>> {
    match (inline, path) {
        (Some(_), Some(_)) => Err(Error::Invalid(
            "pass either --questions or --questions-file, not both".to_owned(),
        )),
        (Some(json), None) => Ok(Some(Source {
            raw: text(json)?,
            flag: "--questions",
        })),
        (None, Some(file)) => Ok(Some(Source {
            raw: read_file(file)?,
            flag: "--questions-file",
        })),
        (None, None) => Ok(None),
    }
}

/// Parses a question set, naming the flag it came from when it will not parse.
fn parse(raw: &str, flag: &str) -> Result<Questions> {
    serde_json::from_str(raw).map_err(|error| {
        Error::Invalid(format!(
            "`{flag}` must be a JSON object of questions keyed by answer name: {error}."
        ))
    })
}

/// The contents of a file, or standard input when the path is `-`.
///
/// # Errors
/// Returns [`Error::Invalid`] when the file cannot be read.
pub fn read_file(path: &str) -> Result<String> {
    if path == STDIN {
        return text(STDIN);
    }
    std::fs::read_to_string(path)
        .map_err(|error| Error::Invalid(format!("could not read {path}: {error}")))
}

/// Items from a JSON array file, or `-` for that array on stdin.
///
/// Line mode cannot carry an item containing a newline, which every real item
/// does — a source file, a document, a diff. This is the way in for those.
///
/// # Errors
/// Returns [`Error::Invalid`] when the file cannot be read, is not an array of
/// strings, or is empty.
pub fn items(path: &str) -> Result<Vec<String>> {
    let raw = read_file(path)?;
    let parsed: Vec<String> = serde_json::from_str(&raw).map_err(|error| {
        Error::Invalid(format!(
            "`--items-file` must be a JSON array of strings: {error}."
        ))
    })?;
    let kept: Vec<String> = parsed
        .into_iter()
        .filter(|i| !i.trim().is_empty())
        .collect();
    if kept.is_empty() {
        return Err(Error::Invalid("`--items-file` held no items".to_owned()));
    }
    Ok(kept)
}

/// The lines of standard input, blank ones dropped.
///
/// # Errors
/// Returns [`Error::Invalid`] when stdin cannot be read or holds no items.
pub fn lines() -> Result<Vec<String>> {
    let raw = text(STDIN)?;
    let items: Vec<String> = raw
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect();
    if items.is_empty() {
        return Err(Error::Invalid(
            "standard input held no items; pass one per line".to_owned(),
        ));
    }
    Ok(items)
}

//! Reading items that already carry a known answer.
//!
//! An eval needs the answer to come from somewhere other than the model being
//! evaluated. That is the whole point, so the labels arrive as data rather than
//! being inferred from anything.

use serde::Deserialize;
use typesafe_sdk_error::{Error, Result};

/// One item and the answer it is already known to have.
#[derive(Deserialize)]
pub(crate) struct Labelled {
    /// The text to ask about.
    pub(crate) item: String,
    /// What the answer should be: `true`/`false` for a noul, a label for a
    /// choice.
    pub(crate) label: serde_json::Value,
}

/// Reads a JSON array of `{item, label}` from a file, or `-` for stdin.
///
/// # Errors
/// Returns [`Error::Invalid`] when the file cannot be read, the JSON is not an
/// array of that shape, or it holds fewer than two distinct labels — a single
/// label cannot be separated from anything.
pub(crate) fn read(path: &str) -> Result<Vec<Labelled>> {
    let raw = typesafe_sdk_cmd_kit::read_file(path)?;
    let rows: Vec<Labelled> = serde_json::from_str(&raw).map_err(|error| {
        Error::Invalid(format!(
            "`--labels-file` must be a JSON array of {{\"item\": \"…\", \"label\": …}}: {error}."
        ))
    })?;
    check(&rows)?;
    Ok(rows)
}

/// Rejects a set nothing could be measured against.
fn check(rows: &[Labelled]) -> Result<()> {
    if rows.len() < 2 {
        return Err(Error::Invalid(
            "an eval needs at least two labelled items".to_owned(),
        ));
    }
    let first = &rows[0].label;
    if rows.iter().all(|r| &r.label == first) {
        return Err(Error::Invalid(
            "every item carries the same label, so nothing can be separated from anything"
                .to_owned(),
        ));
    }
    Ok(())
}

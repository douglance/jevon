//! File length.
//!
//! Two tiers, because one hard limit teaches people to sit at 499 lines. The
//! warning is where a file starts wanting to be split; the denial is where it
//! has stopped being readable.
//!
//! Blank lines and comments do not count. A limit that punishes documentation
//! would buy short files by making them worse.

use std::path::Path;

use crate::allowlist;
use crate::limits::Limits;
use crate::violation::Violation;

const GATE: &str = "file-length";

/// Counts lines that carry code, ignoring blanks and comment-only lines.
///
/// Block comments are tracked so a long `/* ... */` header cannot be used to
/// smuggle in length, and so it is not charged for either.
#[must_use]
pub fn code_line_count(lines: &[&str]) -> usize {
    let mut count = 0;
    let mut in_block = false;

    for line in lines {
        let trimmed = line.trim();
        // Opening a block and continuing one ask the same question: does this
        // line close it?
        if in_block || trimmed.starts_with("/*") {
            in_block = !closes_block(trimmed);
        } else if is_code(trimmed) {
            count += 1;
        }
    }
    count
}

/// A line carries code when it is neither blank nor a line comment.
fn is_code(trimmed: &str) -> bool {
    !trimmed.is_empty() && !trimmed.starts_with("//")
}

/// Whether a line ends an open block comment.
///
/// Trailing code on the closing line is treated as comment, which undercounts by
/// at most one line and keeps this a state machine rather than a tokenizer.
fn closes_block(trimmed: &str) -> bool {
    trimmed.contains("*/")
}

/// Checks one file's length against both tiers.
#[must_use]
pub fn check(path: &Path, source: &str, limits: &Limits) -> Option<Violation> {
    let lines: Vec<&str> = source.lines().collect();
    let count = code_line_count(&lines);

    if count <= limits.file_warn {
        return None;
    }
    // A directive on line 1 covers the file as a whole.
    if allowlist::excused(&lines, 1, GATE) {
        return None;
    }

    let message = format!(
        "{count} lines of code; warn above {}, fail above {}",
        limits.file_warn, limits.file_fail
    );

    if count > limits.file_fail {
        return Some(Violation::deny(GATE, path, None, message));
    }
    Some(Violation::warn(GATE, path, None, message))
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use std::path::Path;

    use super::{check, code_line_count};
    use crate::limits::Limits;
    use crate::violation::Severity;

    #[test]
    fn ignores_blanks_and_line_comments() {
        let lines = ["fn a() {}", "", "// a comment", "   ", "fn b() {}"];
        assert_eq!(code_line_count(&lines), 2);
    }

    #[test]
    fn ignores_block_comments_across_lines() {
        let lines = ["/*", "still a comment", "*/", "fn a() {}"];
        assert_eq!(code_line_count(&lines), 1);
    }

    #[test]
    fn counts_a_single_line_block_comment_as_no_code() {
        let lines = ["/* short */", "fn a() {}"];
        assert_eq!(code_line_count(&lines), 1);
    }

    fn source_of(code_lines: usize) -> String {
        std::iter::repeat_n("fn a() {}", code_lines)
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn passes_under_the_warning_tier() {
        let limits = Limits::default();
        assert!(check(Path::new("x.rs"), &source_of(10), &limits).is_none());
    }

    #[test]
    fn warns_between_the_tiers() {
        let limits = Limits::default();
        let found = check(Path::new("x.rs"), &source_of(limits.file_warn + 1), &limits);
        assert_eq!(found.map(|v| v.severity), Some(Severity::Warn));
    }

    #[test]
    fn denies_above_the_hard_tier() {
        let limits = Limits::default();
        let found = check(Path::new("x.rs"), &source_of(limits.file_fail + 1), &limits);
        assert_eq!(found.map(|v| v.severity), Some(Severity::Deny));
    }

    #[test]
    fn a_reasoned_directive_excuses_the_file() {
        let limits = Limits::default();
        let source = format!(
            "// typesafe-allow file-length reason: one table.\n{}",
            source_of(limits.file_fail + 1)
        );
        assert!(check(Path::new("x.rs"), &source, &limits).is_none());
    }
}

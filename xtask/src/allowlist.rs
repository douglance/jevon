//! The one legitimate way past a size or complexity limit.
//!
//! A limit with a silent escape hatch is not a limit. This one costs a written
//! reason on the line above, which is cheap to add honestly and awkward to add
//! dishonestly — and it leaves the justification where the next reader will
//! look, instead of in a commit message nobody opens.
//!
//! ```text
//! // typesafe-allow file-length reason: the dependency table is one visible block.
//! ```

/// How far above the offending line a directive may sit and still apply.
const LOOKBACK_LINES: usize = 4;

const PREFIX: &str = "typesafe-allow";
const REASON: &str = "reason:";

/// Whether `metric` is excused for the one-based `line`.
///
/// A directive with no stated reason does not count, which is the whole point.
#[must_use]
pub fn excused(source_lines: &[&str], line: usize, metric: &str) -> bool {
    let Some(zero_based) = line.checked_sub(1) else {
        return false;
    };
    let start = zero_based.saturating_sub(LOOKBACK_LINES);

    source_lines
        .get(start..=zero_based.min(source_lines.len().saturating_sub(1)))
        .unwrap_or_default()
        .iter()
        .any(|candidate| mentions(candidate, metric))
}

fn mentions(candidate: &str, metric: &str) -> bool {
    let trimmed = candidate.trim_start();
    if !trimmed.starts_with("//") {
        return false;
    }
    let Some(directive) = trimmed.find(PREFIX).map(|at| &trimmed[at + PREFIX.len()..]) else {
        return false;
    };
    let Some(reason_at) = directive.find(REASON) else {
        return false;
    };
    let names_metric = directive[..reason_at]
        .split_whitespace()
        .any(|word| word == metric);
    let has_reason = !directive[reason_at + REASON.len()..].trim().is_empty();
    names_metric && has_reason
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::excused;

    #[test]
    fn accepts_a_directive_with_a_stated_reason() {
        let lines = [
            "// typesafe-allow file-length reason: one visible table.",
            "fn wide() {}",
        ];
        assert!(excused(&lines, 2, "file-length"));
    }

    #[test]
    fn rejects_a_directive_with_no_reason() {
        let lines = ["// typesafe-allow file-length reason:", "fn wide() {}"];
        assert!(!excused(&lines, 2, "file-length"));
    }

    #[test]
    fn rejects_a_directive_naming_a_different_metric() {
        let lines = [
            "// typesafe-allow nesting reason: unavoidable.",
            "fn wide() {}",
        ];
        assert!(!excused(&lines, 2, "file-length"));
    }

    #[test]
    fn rejects_a_directive_too_far_above() {
        let mut lines = vec!["// typesafe-allow nesting reason: ok."];
        lines.extend(std::iter::repeat_n("", 6));
        lines.push("fn wide() {}");
        let line = lines.len();
        assert!(!excused(&lines, line, "nesting"));
    }

    #[test]
    fn ignores_the_same_words_in_running_code() {
        let lines = [
            r#"let s = "typesafe-allow nesting reason: nope";"#,
            "fn wide() {}",
        ];
        assert!(!excused(&lines, 2, "nesting"));
    }
}

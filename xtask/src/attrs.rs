//! The gate that stops the other gates being switched off.
//!
//! Strict lints are only as strong as the ease of writing `#[allow(...)]` above
//! the problem. This restricts allows to a short list that is defensible in test
//! code, and rejects the rest — so silencing a lint in production code becomes a
//! visible decision rather than a reflex.

use std::path::Path;

use crate::violation::Violation;

const GATE: &str = "allow-attrs";

/// Lints that may be silenced, and only inside test code.
///
/// A test that cannot unwrap is a test written around the lint rather than
/// around the behaviour, so these four are permitted where tests live.
const TEST_ONLY: &[&str] = &["unwrap_used", "expect_used", "panic", "unwrap_in_result"];

/// Flags `#[allow]` / `#[cfg_attr(..., allow)]` outside permitted places.
#[must_use]
pub fn check(path: &Path, source: &str) -> Vec<Violation> {
    if is_test_path(path) {
        return Vec::new();
    }

    let mut in_test_module = false;
    source
        .lines()
        .enumerate()
        .filter_map(|(index, line)| {
            in_test_module |= line.contains("#[cfg(test)]");
            offence(path, line, index + 1, in_test_module)
        })
        .collect()
}

/// The one judgement: an `allow` that this position does not earn.
fn offence(path: &Path, line: &str, number: usize, in_test_module: bool) -> Option<Violation> {
    let silenced = allowed_lint(line)?;
    let permitted = in_test_module && TEST_ONLY.contains(&silenced.as_str());
    (!permitted).then(|| {
        Violation::deny(
            GATE,
            path,
            Some(number),
            format!("`allow({silenced})` outside test code; fix the cause or record a reason"),
        )
    })
}

/// Files under `tests/` are integration tests in their entirety.
fn is_test_path(path: &Path) -> bool {
    path.components()
        .any(|component| component.as_os_str() == "tests")
}

/// Extracts the lint named by an `allow` attribute on this line, if any.
fn allowed_lint(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if !trimmed.starts_with("#[") {
        return None;
    }
    let at = trimmed.find("allow(")?;
    let rest = &trimmed[at + "allow(".len()..];
    let close = rest.find(')')?;
    // `allow(clippy::panic, reason = "…")` names one lint and then explains
    // itself; only the lint before the first comma is the thing being silenced.
    let first = rest[..close].split(',').next()?.trim();
    let name = first.rsplit("::").next()?.trim();
    (!name.is_empty()).then(|| name.to_owned())
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use std::path::Path;

    use super::check;

    fn count(source: &str) -> usize {
        check(Path::new("crates/board-core/src/lib.rs"), source).len()
    }

    #[test]
    fn production_code_may_not_silence_a_lint() {
        assert_eq!(count("#[allow(clippy::unwrap_used)]\nfn f() {}"), 1);
    }

    #[test]
    fn test_modules_may_silence_the_permitted_few() {
        let source =
            "#[cfg(test)]\nmod tests {\n    #[allow(clippy::unwrap_used)]\n    fn t() {}\n}";
        assert_eq!(count(source), 0);
    }

    #[test]
    fn test_modules_may_not_silence_anything_else() {
        let source =
            "#[cfg(test)]\nmod tests {\n    #[allow(clippy::too_many_lines)]\n    fn t() {}\n}";
        assert_eq!(count(source), 1);
    }

    #[test]
    fn integration_test_files_are_exempt_entirely() {
        let found = check(
            Path::new("crates/board-core/tests/e2e.rs"),
            "#[allow(clippy::panic)]",
        );
        assert!(found.is_empty());
    }

    /// A reason is encouraged, and must not be mistaken for the lint name.
    #[test]
    fn a_reason_does_not_hide_the_lint_being_silenced() {
        let source =
            "#[cfg(test)]\n#[allow(clippy::panic, reason = \"tests fail loudly\")]\nmod t {}";
        assert!(check(Path::new("src/lib.rs"), source).is_empty());

        let banned =
            "#[cfg(test)]\n#[allow(clippy::wildcard_imports, reason = \"convenient\")]\nmod t {}";
        assert_eq!(check(Path::new("src/lib.rs"), banned).len(), 1);
    }

    #[test]
    fn a_mention_in_prose_is_not_an_attribute() {
        assert_eq!(
            count("// we could allow(clippy::unwrap_used) here but do not\n"),
            0
        );
    }
}

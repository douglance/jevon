//! Making sure every crate is actually governed.
//!
//! `[workspace.lints]` does nothing for a member that does not opt in with
//! `[lints] workspace = true`. A new crate created by copy-paste is the usual
//! way a corner of the tree quietly escapes the lint set, so this checks that
//! every member opted in — and that shared metadata is inherited rather than
//! drifting per crate.

use std::path::Path;

use anyhow::Result;

use crate::discover::{self, CrateDir};
use crate::violation::Violation;

const GATE: &str = "manifest";

/// Fields every member must inherit from the workspace rather than restate.
const INHERITED: &[&str] = &[
    "version",
    "edition",
    "rust-version",
    "license",
    "repository",
];

/// Checks lint opt-in and metadata inheritance for every member.
///
/// # Errors
/// Fails if the workspace cannot be listed or a manifest cannot be parsed.
pub fn check(root: &Path) -> Result<Vec<Violation>> {
    let crates = discover::crates(root)?;
    let mut violations = Vec::new();
    for krate in &crates {
        violations.extend(check_one(krate)?);
    }
    Ok(violations)
}

fn check_one(krate: &CrateDir) -> Result<Vec<Violation>> {
    let manifest = discover::manifest(&krate.manifest)?;
    let mut violations = Vec::new();

    if !inherits_lints(&manifest) {
        violations.push(Violation::deny(
            GATE,
            &krate.manifest,
            None,
            format!("{} is missing `[lints] workspace = true`", krate.name),
        ));
    }
    violations.extend(missing_inherited(krate, &manifest));
    Ok(violations)
}

fn inherits_lints(manifest: &toml::Value) -> bool {
    manifest
        .get("lints")
        .and_then(|lints| lints.get("workspace"))
        .and_then(toml::Value::as_bool)
        .unwrap_or(false)
}

fn missing_inherited(krate: &CrateDir, manifest: &toml::Value) -> Vec<Violation> {
    let Some(package) = manifest.get("package") else {
        return Vec::new();
    };
    INHERITED
        .iter()
        .filter(|field| !is_inherited(package, field))
        .map(|field| {
            Violation::deny(
                GATE,
                &krate.manifest,
                None,
                format!("{} should declare `{field}.workspace = true`", krate.name),
            )
        })
        .collect()
}

/// True when the field is present and spelled `field.workspace = true`.
fn is_inherited(package: &toml::Value, field: &str) -> bool {
    package
        .get(field)
        .and_then(|value| value.get("workspace"))
        .and_then(toml::Value::as_bool)
        .unwrap_or(false)
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::{inherits_lints, is_inherited};

    fn parse(text: &str) -> toml::Value {
        toml::from_str(text).expect("test fixture should parse")
    }

    #[test]
    fn detects_the_lint_opt_in() {
        assert!(inherits_lints(&parse("[lints]\nworkspace = true\n")));
    }

    #[test]
    fn a_missing_lints_table_is_not_opted_in() {
        assert!(!inherits_lints(&parse("[package]\nname = \"x\"\n")));
    }

    #[test]
    fn an_explicit_false_is_not_opted_in() {
        assert!(!inherits_lints(&parse("[lints]\nworkspace = false\n")));
    }

    #[test]
    fn detects_an_inherited_field() {
        let manifest = parse("[package]\nversion = { workspace = true }\n");
        let package = manifest.get("package").expect("package table");
        assert!(is_inherited(package, "version"));
    }

    #[test]
    fn a_restated_literal_is_not_inherited() {
        let manifest = parse("[package]\nversion = \"0.1.0\"\n");
        let package = manifest.get("package").expect("package table");
        assert!(
            !is_inherited(package, "version"),
            "a literal silently drifts from the workspace"
        );
    }
}

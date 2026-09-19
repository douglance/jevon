//! Structure, size and complexity gates for the jev command-line workspace.
//!
//! These exist because the alternative does not work. Every repository on this
//! machine that stayed small has a gate like this; the ones that did not have
//! files of three thousand lines and permission checks scattered through them.
//! Intent does not survive a deadline. A failing build does.
//!
//! Gates are pure: they take the tree and return [`Violation`]s. Printing and
//! exiting happen once, at the edge, which keeps every rule testable.

pub mod allowlist;
pub mod attrs;
pub mod complexity;
pub mod cycles;
pub mod deps;
pub mod discover;
pub mod layers;
pub mod limits;
pub mod manifests;
pub mod metrics;
pub mod size;
pub mod units;
pub mod violation;

use std::path::Path;

use anyhow::{Context, Result};

use crate::limits::Limits;
use crate::violation::Violation;

/// Runs every gate over the workspace rooted at `root`.
///
/// # Errors
/// Fails if the tree cannot be read; rule breaches are returned, not errors.
pub fn check(root: &Path) -> Result<Vec<Violation>> {
    let limits = Limits::default();
    let mut violations = Vec::new();

    violations.extend(check_sources(root, &limits)?);
    violations.extend(deps::check(root)?);
    violations.extend(manifests::check(root)?);

    violations.sort_by(|a, b| {
        b.severity
            .cmp(&a.severity)
            .then_with(|| a.file.cmp(&b.file))
    });
    Ok(violations)
}

/// Runs the per-file gates over every Rust source in the workspace.
///
/// # Errors
/// Fails if a source file cannot be read.
fn check_sources(root: &Path, limits: &Limits) -> Result<Vec<Violation>> {
    let mut violations = Vec::new();

    for path in discover::rust_files(root)? {
        let source = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;

        violations.extend(size::check(&path, &source, limits));
        violations.extend(complexity::check(&path, &source, limits));
        violations.extend(attrs::check(&path, &source));
    }
    Ok(violations)
}

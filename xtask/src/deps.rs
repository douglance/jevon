//! Crate boundaries, as an executable rule rather than an intention.
//!
//! Two rules, checked against every declared dependency: a crate may depend
//! only on its own layer or below, and the graph must stay acyclic. The layer
//! table lives in [`crate::layers`]; the cycle walk in [`crate::cycles`].

use std::collections::BTreeSet;
use std::path::Path;

use anyhow::Result;

use crate::cycles::{self, Graph};
use crate::discover::{self, CrateDir};
use crate::layers::{EXCLUSIVE, FORBIDDEN, layer_of};
use crate::violation::Violation;

const GATE: &str = "deps";

/// Checks layer direction, leaf purity, exclusivity and cycles.
///
/// # Errors
/// Fails if the workspace cannot be listed or a manifest cannot be parsed.
pub fn check(root: &Path) -> Result<Vec<Violation>> {
    let crates = discover::crates(root)?;
    let mut violations = Vec::new();
    let mut graph: Graph = Graph::new();

    for krate in &crates {
        let deps = dependencies(krate)?;
        violations.extend(check_crate(krate, &deps));
        graph.insert(krate.name.clone(), internal_only(&deps));
    }
    violations.extend(cycles::cycles(root, &graph));
    Ok(violations)
}

fn check_crate(krate: &CrateDir, deps: &BTreeSet<String>) -> Vec<Violation> {
    let mut violations = Vec::new();
    let Some(own_layer) = layer_of(&krate.name) else {
        violations.push(Violation::deny(
            GATE,
            &krate.manifest,
            None,
            format!(
                "{} is not placed in the layer map in xtask/src/layers.rs",
                krate.name
            ),
        ));
        return violations;
    };

    for dep in deps {
        violations.extend(check_edge(krate, own_layer, dep));
    }
    violations
}

fn check_edge(krate: &CrateDir, own_layer: u8, dep: &str) -> Vec<Violation> {
    [
        transport_purity(krate, dep),
        exclusivity(krate, dep),
        direction(krate, own_layer, dep),
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// The SDK owns the wire; a command holding its own HTTP client is a design error.
fn transport_purity(krate: &CrateDir, dep: &str) -> Option<Violation> {
    FORBIDDEN.contains(&dep).then(|| {
        deny(
            krate,
            format!(
                "{} must not depend on {dep}; the SDK owns the transport",
                krate.name
            ),
        )
    })
}

/// Some crates are owned by exactly one façade.
fn exclusivity(krate: &CrateDir, dep: &str) -> Option<Violation> {
    let &(_, owner) = EXCLUSIVE.iter().find(|&&(name, _)| name == dep)?;
    (owner != krate.name).then(|| {
        deny(
            krate,
            format!("only {owner} may depend on {dep}, not {}", krate.name),
        )
    })
}

/// A crate may depend on its own layer or below, never above.
fn direction(krate: &CrateDir, own_layer: u8, dep: &str) -> Option<Violation> {
    let dep_layer = layer_of(dep)?;
    (dep_layer > own_layer).then(|| {
        deny(
            krate,
            format!(
                "{} (layer {own_layer}) depends upward on {dep} (layer {dep_layer})",
                krate.name
            ),
        )
    })
}

fn deny(krate: &CrateDir, message: String) -> Violation {
    Violation::deny(GATE, &krate.manifest, None, message)
}

fn internal_only(deps: &BTreeSet<String>) -> BTreeSet<String> {
    deps.iter()
        .filter(|dep| layer_of(dep).is_some())
        .cloned()
        .collect()
}

/// Every dependency name across normal, dev and build tables.
fn dependencies(krate: &CrateDir) -> Result<BTreeSet<String>> {
    let manifest = discover::manifest(&krate.manifest)?;
    let mut names = BTreeSet::new();
    for table in ["dependencies", "dev-dependencies", "build-dependencies"] {
        if let Some(entries) = manifest.get(table).and_then(toml::Value::as_table) {
            names.extend(entries.keys().cloned());
        }
    }
    Ok(names)
}

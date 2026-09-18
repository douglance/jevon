//! Cycle detection over the internal dependency graph.
//!
//! Same-layer edges are allowed, because peers within a layer legitimately
//! compose. This is the check that stops that becoming a knot.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::violation::Violation;

const GATE: &str = "deps";

/// Internal dependency edges, crate name to the crates it depends on.
pub type Graph = BTreeMap<String, BTreeSet<String>>;

/// Reports a cycle if one exists, by depth-first search over internal edges.
#[must_use]
pub fn cycles(root: &Path, graph: &Graph) -> Vec<Violation> {
    let mut settled = BTreeSet::new();
    let mut stack = BTreeSet::new();

    for node in graph.keys() {
        if let Some(found) = walk(node, graph, &mut settled, &mut stack) {
            return vec![Violation::deny(
                GATE,
                &root.join("Cargo.toml"),
                None,
                format!("dependency cycle through {found}"),
            )];
        }
    }
    Vec::new()
}

fn walk(
    node: &str,
    graph: &Graph,
    settled: &mut BTreeSet<String>,
    stack: &mut BTreeSet<String>,
) -> Option<String> {
    if settled.contains(node) {
        return None;
    }
    if !stack.insert(node.to_owned()) {
        return Some(node.to_owned());
    }
    let found = graph
        .get(node)
        .into_iter()
        .flatten()
        .find_map(|next| walk(next, graph, settled, stack));
    stack.remove(node);
    settled.insert(node.to_owned());
    found
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{Graph, cycles};

    /// A literal edge list, as the tests spell it.
    type Edges<'a> = [(&'a str, &'a [&'a str])];

    fn graph_of(edges: &Edges<'_>) -> Graph {
        edges
            .iter()
            .map(|(from, to)| {
                (
                    (*from).to_owned(),
                    to.iter().map(|t| (*t).to_owned()).collect(),
                )
            })
            .collect()
    }

    #[test]
    fn an_acyclic_graph_is_accepted() {
        let graph = graph_of(&[("a", &["b"]), ("b", &["c"]), ("c", &[])]);
        assert!(cycles(Path::new("/tmp"), &graph).is_empty());
    }

    /// Asserting only that something was reported would pass for a check that
    /// always complains, so this pins the message to a crate in the cycle. The
    /// names are deliberately distinctive: single letters would be matched by
    /// almost any English sentence, including a hardcoded one.
    #[test]
    fn a_cycle_is_reported_and_names_a_crate_in_it() {
        let graph = graph_of(&[
            ("zeta-store", &["kappa-render"]),
            ("kappa-render", &["omega-parse"]),
            ("omega-parse", &["zeta-store"]),
        ]);
        let found = cycles(Path::new("/tmp"), &graph);
        assert_eq!(found.len(), 1);
        let message = &found[0].message;
        assert!(
            ["zeta-store", "kappa-render", "omega-parse"]
                .iter()
                .any(|node| message.contains(node)),
            "the cycle report names no crate in it: {message}"
        );
    }

    #[test]
    fn a_diamond_is_not_a_cycle() {
        let graph = graph_of(&[("a", &["b", "c"]), ("b", &["d"]), ("c", &["d"]), ("d", &[])]);
        assert!(cycles(Path::new("/tmp"), &graph).is_empty());
    }
}

//! Where each crate sits, and what it may not touch.
//!
//! Splitting a system into many small crates only helps if the dependency
//! direction holds. Without a table, a command crate acquires a socket one
//! convenient import at a time and the boundaries become decoration. Adding a
//! crate means placing it here; a crate absent from this table fails the build.

/// The layer each crate belongs to. Adding a crate means placing it here.
pub(crate) const LAYERS: &[(&str, u8)] = &[
    // 0 — the shared command kit: argument reading, client construction,
    //     output shaping. Everything a command needs and no command owns.
    ("typesafe-sdk-cmd-kit", 0),
    // 1 — one crate per command. Siblings, never aware of each other.
    ("typesafe-sdk-cmd-ask", 1),
    ("typesafe-sdk-cmd-classify", 1),
    ("typesafe-sdk-cmd-eval", 1),
    ("typesafe-sdk-cmd-doctor", 1),
    ("typesafe-sdk-cmd-models", 1),
    // 2 — the binary that assembles them, and the gates.
    ("jevon", 2),
    ("xtask", 2),
];

/// Third-party crates no crate here may name.
///
/// The SDK owns the wire. A command that reached for an HTTP client directly
/// would get its own retry, timeout and header behaviour, which is how two code
/// paths quietly stop agreeing about what a request does.
pub(crate) const FORBIDDEN: &[&str] = &["reqwest", "hyper"];

/// Crates only one named crate may depend on.
///
/// One crate builds the transport and the client. Anything else naming
/// `typesafe-sdk-http` means a command grew its own way to reach the API, which
/// defeats the point of a shared kit.
pub(crate) const EXCLUSIVE: &[(&str, &str)] = &[("typesafe-sdk-http", "typesafe-sdk-cmd-kit")];

/// The layer `name` belongs to, if it is a workspace member.
#[must_use]
pub fn layer_of(name: &str) -> Option<u8> {
    LAYERS
        .iter()
        .find(|&&(krate, _)| krate == name)
        .map(|&(_, layer)| layer)
}

#[cfg(test)]
mod tests {
    use super::{LAYERS, layer_of};

    #[test]
    fn the_kit_sits_below_the_commands() {
        assert!(layer_of("typesafe-sdk-cmd-kit") < layer_of("typesafe-sdk-cmd-ask"));
    }

    #[test]
    fn the_commands_sit_below_the_binary() {
        assert!(layer_of("typesafe-sdk-cmd-ask") < layer_of("jevon"));
    }

    #[test]
    fn every_layer_is_populated() {
        for layer in 0..=2u8 {
            assert!(
                LAYERS.iter().any(|&(_, l)| l == layer),
                "layer {layer} has no crates"
            );
        }
    }

    #[test]
    fn no_crate_is_listed_twice() {
        let mut names: Vec<&str> = LAYERS.iter().map(|&(n, _)| n).collect();
        names.sort_unstable();
        let count = names.len();
        names.dedup();
        assert_eq!(names.len(), count, "a crate appears twice in the layer map");
    }
}

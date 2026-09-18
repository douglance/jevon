//! Where each crate sits, and what it may not touch.
//!
//! Splitting a system into many small crates only helps if the dependency
//! direction holds. Without a table, layer 0 acquires a socket one convenient
//! import at a time and the boundaries become decoration. Adding a crate means
//! placing it here; a crate absent from this table fails the build.

/// The layer each crate belongs to. Adding a crate means placing it here.
// typesafe-allow file-length reason: the layer map is the architecture, and is far
// clearer read as one uninterrupted table than split across modules.
pub(crate) const LAYERS: &[(&str, u8)] = &[
    // 0 — leaf. Pure values and decisions, no I/O.
    ("typesafe-sdk-error", 0),
    ("typesafe-sdk-env", 0),
    ("typesafe-sdk-runtime", 0),
    ("typesafe-sdk-headers", 0),
    // 1 — domain. The wire vocabulary and the reliability policy.
    ("typesafe-sdk-questions", 1),
    ("typesafe-sdk-answers", 1),
    ("typesafe-sdk-models", 1),
    ("typesafe-sdk-retry", 1),
    ("typesafe-sdk-log", 1),
    // 2 — proc macro. No reverse dependency on the runtime crates.
    ("typesafe-sdk-derive", 2),
    // 3 — I/O and configuration.
    ("typesafe-sdk-http", 3),
    ("typesafe-sdk-config", 3),
    // 4 — the SDK facade consumers depend on.
    ("typesafe-sdk-client", 4),
    // 5 — command surface, one crate per command.
    ("typesafe-sdk-cmd-kit", 5),
    ("typesafe-sdk-cmd-models", 5),
    ("typesafe-sdk-cmd-ask", 5),
    ("typesafe-sdk-cmd-classify", 5),
    ("typesafe-sdk-cmd-doctor", 5),
    // 6 — binaries and cross-crate tests.
    ("jevon", 6),
    ("typesafe-sdk-tests", 6),
    ("xtask", 6),
];

/// Third-party crates a layer-0 crate may never name.
///
/// Keeping I/O out of the leaf is what lets its tests run in microseconds and
/// what stops wire vocabulary quietly acquiring a socket.
pub(crate) const LEAF_FORBIDDEN: &[&str] = &["reqwest", "tokio", "incurs", "hyper"];

/// Third-party crates only one named crate may depend on.
///
/// One crate owns the HTTP client. Anything else naming it directly means a
/// layer has grown its own transport, which is how retry and timeout behaviour
/// quietly diverges between two code paths.
pub(crate) const EXCLUSIVE: &[(&str, &str)] = &[("reqwest", "typesafe-sdk-http")];

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
    fn the_leaf_sits_below_the_transport() {
        assert!(layer_of("typesafe-sdk-error") < layer_of("typesafe-sdk-http"));
    }

    #[test]
    fn the_client_sits_below_the_commands() {
        assert!(layer_of("typesafe-sdk-client") < layer_of("typesafe-sdk-cmd-ask"));
    }

    #[test]
    fn every_layer_is_populated() {
        for layer in 0..=6u8 {
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

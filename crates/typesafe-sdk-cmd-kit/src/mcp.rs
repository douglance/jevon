//! How each command presents itself to an agent.
//!
//! Without these, incurs assumes the worst and routes every command through
//! its write path, which an agent client gates behind approval. Saying that
//! listing models is read-only is what lets an agent call it freely — and
//! saying `ask` reaches the network is what stops it being treated as local.

use incurs::command::{McpAnnotations, McpCommandOptions};

/// A command that reads and does not reach outside the process.
#[must_use]
pub fn read_only(title: &str) -> McpCommandOptions {
    McpCommandOptions {
        annotations: Some(McpAnnotations {
            title: Some(title.to_owned()),
            read_only_hint: Some(true),
            destructive_hint: Some(false),
            idempotent_hint: Some(true),
            open_world_hint: Some(false),
        }),
        ..McpCommandOptions::default()
    }
}

/// A command that reads, but does so by calling the API.
///
/// Read-only because nothing is modified, open-world because the answer comes
/// from a service and two identical calls may differ.
#[must_use]
pub fn read_only_remote(title: &str) -> McpCommandOptions {
    McpCommandOptions {
        annotations: Some(McpAnnotations {
            title: Some(title.to_owned()),
            read_only_hint: Some(true),
            destructive_hint: Some(false),
            idempotent_hint: Some(false),
            open_world_hint: Some(true),
        }),
        ..McpCommandOptions::default()
    }
}

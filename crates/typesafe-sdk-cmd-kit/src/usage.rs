//! What a run cost, in the shape the CLI reports it.
//!
//! The SDK's `Usage` is a wire type and carries no JSON Schema, so the command
//! surface needs its own. It lives here rather than in one command because two
//! commands report it and a schema that says `input_tokens` in one place and
//! something else in another is the drift this workspace exists to prevent.

use schemars::JsonSchema;
use serde::Serialize;

/// Tokens consumed.
#[derive(Default, Clone, Copy, Serialize, JsonSchema)]
pub struct Usage {
    /// Input tokens used.
    pub input_tokens: u64,
    /// Output tokens used.
    pub output_tokens: u64,
}

impl Usage {
    /// Adds one response's tokens to the running total.
    ///
    /// Saturating rather than wrapping: a batch large enough to overflow a
    /// `u64` cannot happen, but reporting a total that went backwards would be
    /// worse than reporting one that stopped climbing.
    pub fn add(&mut self, other: &typesafe_sdk_answers::Usage) {
        self.input_tokens = self.input_tokens.saturating_add(other.input_tokens);
        self.output_tokens = self.output_tokens.saturating_add(other.output_tokens);
    }
}

impl From<&typesafe_sdk_answers::Usage> for Usage {
    fn from(usage: &typesafe_sdk_answers::Usage) -> Self {
        let mut total = Self::default();
        total.add(usage);
        total
    }
}

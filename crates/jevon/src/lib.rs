//! The `typesafe` command-line interface.
//!
//! One command graph serves every surface: the terminal, `--schema`,
//! `--llms-full`, shell completions, an MCP server over stdio, and agent skill
//! files. Adding a command here adds it to all of them at once, which is why
//! the command crates return definitions rather than printing anything.

use incurs::cli::Cli;

/// Builds the command graph.
///
/// Exposed so tests can drive it in-process through `serve_to`, which is the
/// same path the binary takes — a test cannot drift from the process.
#[must_use]
pub fn build_cli() -> Cli {
    Cli::create("jev")
        .aliases(vec!["typesafe".to_owned()])
        .version(env!("CARGO_PKG_VERSION"))
        .description("Ask TypeSafe questions about text and get typed answers")
        .command("ask", typesafe_sdk_cmd_ask::command())
        .command("classify", typesafe_sdk_cmd_classify::command())
        .command("eval", typesafe_sdk_cmd_eval::command())
        .command("doctor", typesafe_sdk_cmd_doctor::command())
        .group(models_group())
}

/// The `models` namespace.
fn models_group() -> Cli {
    Cli::create("models")
        .description("Inspect the models available to this account")
        .command("list", typesafe_sdk_cmd_models::list())
}

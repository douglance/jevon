//! The `models` command.

use incurs::command::{CommandDef, TypedContext, TypedResult};
use schemars::JsonSchema;
use serde::Serialize;
use typesafe_sdk_cmd_kit::{client, code_for, read_only_remote};

/// One model, as the CLI reports it.
#[derive(Serialize, JsonSchema)]
pub struct Model {
    /// The name to pass as `--model`.
    pub name: String,
    /// What the model is.
    pub description: String,
    /// When it was released.
    pub release_date: String,
}

/// Everything the account can use.
#[derive(Serialize, JsonSchema)]
pub struct Models {
    /// The available models, newest listing first.
    pub models: Vec<Model>,
}

/// Builds the `models list` command.
#[must_use]
pub fn list() -> CommandDef {
    CommandDef::typed::<(), (), (), Models, _, _>("list", |_ctx: TypedContext<(), (), ()>| async {
        match client() {
            Err(error) => TypedResult::error(code_for(&error), error.to_string()),
            Ok(client) => match client.models().await {
                Err(error) => TypedResult::error(code_for(&error), error.to_string()),
                Ok(cards) => TypedResult::ok(Models {
                    models: cards
                        .into_iter()
                        .map(|card| Model {
                            name: card.name,
                            description: card.description,
                            release_date: card.release_date,
                        })
                        .collect(),
                }),
            },
        }
    })
    .description("List the models available to this account")
    .mcp(read_only_remote("List models"))
    .done()
}

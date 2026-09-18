//! The `models` command.

use incurs::command::{CommandDef, Example, TypedContext, TypedResult};
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

/// Reshapes the SDK's cards into what the CLI reports.
fn reported(cards: Vec<typesafe_sdk_models::ModelCard>) -> Models {
    Models {
        models: cards
            .into_iter()
            .map(|card| Model {
                name: card.name,
                description: card.description,
                release_date: card.release_date,
            })
            .collect(),
    }
}

/// Builds the `models list` command.
#[must_use]
pub fn list() -> CommandDef {
    CommandDef::typed::<(), (), (), Models, _, _>("list", |_ctx: TypedContext<(), (), ()>| async {
        match client() {
            Err(error) => TypedResult::error(code_for(&error), error.to_string()),
            Ok(client) => match client.models().await {
                Err(error) => TypedResult::error(code_for(&error), error.to_string()),
                Ok(cards) => TypedResult::ok(reported(cards)),
            },
        }
    })
    .description("List the models available to this account")
    .hint(
        "`jev-latest` is an alias that resolves to a concrete version, so an answer \
         reports something like `jev-1.13.0` rather than the name you asked for. Pin \
         the resolved version with --model when you need two runs to be comparable.",
    )
    .examples(vec![Example {
        command: "--json".to_owned(),
        description: Some("List every model this key can use".to_owned()),
    }])
    .mcp(read_only_remote("List models"))
    .done()
}

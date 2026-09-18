//! The `doctor` command.

use incurs::command::{CommandDef, TypedContext, TypedResult};
use schemars::JsonSchema;
use serde::Serialize;
use typesafe_sdk_cmd_kit::read_only;
use typesafe_sdk_config::Builder;
use typesafe_sdk_env::{Process, Var, read};

/// What the SDK resolved, and where each setting came from.
#[derive(Serialize, JsonSchema)]
pub struct Diagnosis {
    /// Whether a usable API key was found.
    pub authenticated: bool,
    /// The key, masked. Absent when no key is configured.
    pub api_key: Option<String>,
    /// The API root that will be used.
    pub base_url: String,
    /// The model used when a request does not name one.
    pub default_model: String,
    /// The per-attempt timeout, in milliseconds.
    pub timeout_ms: u64,
    /// Which environment variables are set.
    pub environment: Vec<EnvVar>,
    /// Anything that would stop a request working.
    pub problems: Vec<String>,
}

/// One environment variable and whether it is set.
#[derive(Serialize, JsonSchema)]
pub struct EnvVar {
    /// The variable's name.
    pub name: String,
    /// Whether it holds a non-blank value.
    pub set: bool,
}

/// Builds the `doctor` command.
#[must_use]
pub fn command() -> CommandDef {
    CommandDef::typed::<(), (), (), Diagnosis, _, _>(
        "doctor",
        |_ctx: TypedContext<(), (), ()>| async { TypedResult::ok(diagnose()) },
    )
    .description("Report the resolved configuration and anything that would stop it working")
    .mcp(read_only("Diagnose configuration"))
    .done()
}

/// Resolves the configuration without sending anything.
fn diagnose() -> Diagnosis {
    let environment = Var::all()
        .into_iter()
        .map(|var| EnvVar {
            name: var.name().to_owned(),
            set: read(&Process, var).is_some(),
        })
        .collect();

    match Builder::new().build(&Process) {
        Ok(config) => Diagnosis {
            authenticated: true,
            api_key: Some(config.key_hint()),
            base_url: config.base_url.clone(),
            default_model: config.default_model.clone(),
            timeout_ms: config.timeout_ms,
            environment,
            problems: Vec::new(),
        },
        Err(error) => Diagnosis {
            authenticated: false,
            api_key: None,
            base_url: typesafe_sdk_config::DEFAULT_BASE_URL.to_owned(),
            default_model: typesafe_sdk_config::DEFAULT_MODEL.to_owned(),
            timeout_ms: 0,
            environment,
            problems: vec![error.to_string()],
        },
    }
}

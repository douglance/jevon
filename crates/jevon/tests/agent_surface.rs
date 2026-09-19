//! How the command graph presents itself to an agent.
//!
//! MCP clients decide what they may call without asking from the annotations
//! a tool advertises. An unannotated command is assumed to write, which sends a
//! harmless read through an approval prompt.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "a test that cannot fail loudly is not a test"
)]
#![allow(
    clippy::future_not_send,
    reason = "serve_to writes through a non-Send &mut dyn Write, as the binary does"
)]

use jevon::build_cli;

/// Runs the CLI in agent mode and returns its exit code and stdout.
async fn observe(argv: &[&str]) -> (Option<i32>, String) {
    let mut out = Vec::new();
    let exit = build_cli()
        .serve_to(
            argv.iter().map(|a| (*a).to_owned()).collect(),
            &mut out,
            false,
        )
        .await
        .expect("the command graph must run");
    (exit, String::from_utf8(out).expect("stdout must be UTF-8"))
}

/// Without annotations, incurs assumes the worst and routes every command
/// through its write path, which an agent client gates behind approval.
#[tokio::test]
async fn every_tool_declares_how_it_behaves() {
    let cli = build_cli();
    for tool in cli.tool_catalog().definitions() {
        let annotations = tool
            .annotations
            .as_ref()
            .unwrap_or_else(|| panic!("{} declares no MCP annotations", tool.name));
        assert_eq!(
            annotations.read_only_hint,
            Some(true),
            "{} is not marked read-only; every command here only reads",
            tool.name
        );
        assert_eq!(annotations.destructive_hint, Some(false), "{}", tool.name);
    }
}

/// `ask` and `models list` reach the API; `doctor` reads local configuration.
#[tokio::test]
async fn commands_that_reach_the_network_say_so() {
    let cli = build_cli();
    let catalog = cli.tool_catalog();
    let open_world = |name: &str| {
        catalog
            .get(name)
            .and_then(|tool| tool.annotations.as_ref())
            .and_then(|a| a.open_world_hint)
    };
    assert_eq!(open_world("ask"), Some(true));
    assert_eq!(open_world("models_list"), Some(true));
    assert_eq!(open_world("doctor"), Some(false));
}

/// `--format yaml` must emit YAML, not silently fall back to JSON.
#[tokio::test]
async fn yaml_output_is_actually_yaml() {
    let (_, out) = observe(&["doctor", "--format", "yaml"]).await;
    assert!(
        out.starts_with("authenticated:"),
        "expected YAML, got:\n{out}"
    );
}

/// Token budgeting is what an agent uses to stay inside a context window, so
/// these must keep working. Declaring config files in incurs 0.7.0 silently
/// breaks them; see douglance/incurs fix/config-flag-swallows-builtins.
#[tokio::test]
async fn the_token_budgeting_flags_are_reachable() {
    for argv in [
        vec!["doctor", "--token-count"],
        vec!["doctor", "--token-limit", "50"],
        vec!["doctor", "--token-offset", "1"],
        vec!["doctor", "--filter-output", "base_url"],
    ] {
        let (exit, _) = observe(&argv).await;
        assert_eq!(exit, None, "{argv:?} was rejected");
    }
}

/// A schema tells an agent what it *may* send. It cannot say which primitive
/// suits a question, or that a noul near 0.5 means ambiguity rather than a
/// middling amount. Examples and hints carry that, and both reach the skill
/// files an agent actually reads.
#[tokio::test]
async fn every_command_carries_worked_examples() {
    let (_, out) = observe(&["--llms-full", "--format", "json"]).await;
    let manifest: serde_json::Value = serde_json::from_str(&out).unwrap();

    for command in ["ask", "classify", "doctor", "eval", "models list"] {
        let found = manifest["commands"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["name"] == command)
            .unwrap_or_else(|| panic!("{command} missing from the manifest"));
        let examples = found["examples"].as_array().map_or(&[][..], Vec::as_slice);
        assert!(!examples.is_empty(), "{command} has no examples");
        let described = examples
            .iter()
            .filter(|e| e["description"].as_str().is_some_and(|d| !d.is_empty()))
            .count();
        assert_eq!(
            described,
            examples.len(),
            "{command} has an example that does not say what it demonstrates"
        );
    }
}

/// Hints do not reach `--llms-full`, only `--help` and the skill file, so this
/// asserts them where they appear. Each command is paired with the point its
/// hint exists to make; deleting the guidance breaks the test.
#[tokio::test]
async fn every_command_explains_what_the_schema_cannot() {
    let expected = [
        (vec!["ask"], "a noul near 0.5"),
        (vec!["ask"], "docs.typesafe.ai/llms.txt"),
        (vec!["classify"], "one client, one connection"),
        (vec!["models", "list"], "resolves to a concrete version"),
        (vec!["doctor"], "needs that variable in its own environment"),
        (vec!["doctor"], "only command here that needs no"),
    ];

    for (command, point) in expected {
        let mut argv = command.clone();
        argv.push("--help");
        let (_, out) = observe(&argv).await;
        assert!(
            out.contains(point),
            "{command:?} no longer explains {point:?}:\n{out}"
        );
    }
}

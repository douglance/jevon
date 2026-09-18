//! The command graph's contract, checked in-process.
//!
//! `serve_to` is the same path the binary takes, so these cannot drift from
//! the real process. The tool-catalog assertions matter more than they look:
//! they are what catches a command that skipped the typed path and so reaches
//! MCP without a description or an output schema.
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

#[tokio::test]
async fn every_command_is_exposed_as_a_tool() {
    let cli = build_cli();
    let mut names: Vec<String> = cli
        .tool_catalog()
        .definitions()
        .iter()
        .map(|tool| tool.name.clone())
        .collect();
    names.sort();
    assert_eq!(names, vec!["ask", "doctor", "models_list"]);
}

#[tokio::test]
async fn every_tool_declares_a_description_and_an_output_schema() {
    let cli = build_cli();
    for tool in cli.tool_catalog().definitions() {
        assert!(
            !tool.description.is_empty(),
            "{} has no description",
            tool.name
        );
        assert!(
            tool.output_schema.is_some(),
            "{} publishes no output schema, so it never went through the typed path",
            tool.name
        );
    }
}

#[tokio::test]
async fn the_help_lists_every_command() {
    let (exit, out) = observe(&["--help"]).await;
    assert_eq!(exit, None);
    for command in ["ask", "doctor", "models"] {
        assert!(
            out.contains(command),
            "`{command}` missing from help:\n{out}"
        );
    }
}

#[tokio::test]
async fn the_ask_schema_publishes_its_arguments_and_options() {
    let (_, out) = observe(&["ask", "--schema", "--json"]).await;
    let schema: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(schema["args"]["properties"]["state"]["type"], "string");
    for flag in ["questions", "choice", "score", "noul", "model"] {
        assert!(
            !schema["options"]["properties"][flag].is_null(),
            "--{flag} missing from the schema:\n{out}"
        );
    }
}

/// A question set is required, and saying so is the CLI's job rather than
/// serde's.
#[tokio::test]
async fn asking_nothing_is_a_validation_error_not_a_parse_failure() {
    let (exit, out) = observe(&["ask", "some text", "--json"]).await;
    assert_eq!(exit, Some(1));
    let reported: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(reported["code"], "VALIDATION_ERROR");
    assert!(
        reported["message"]
            .as_str()
            .unwrap()
            .starts_with("No questions were given."),
        "{out}"
    );
}

#[tokio::test]
async fn a_malformed_questions_payload_names_the_flag() {
    let (exit, out) = observe(&["ask", "text", "--questions", "not json", "--json"]).await;
    assert_eq!(exit, Some(1));
    let reported: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(reported["code"], "VALIDATION_ERROR");
    assert!(
        reported["message"]
            .as_str()
            .unwrap()
            .contains("--questions"),
        "{out}"
    );
}

#[tokio::test]
async fn an_unknown_command_fails_rather_than_guessing() {
    let (exit, _) = observe(&["nonexistent"]).await;
    assert_eq!(exit, Some(1));
}

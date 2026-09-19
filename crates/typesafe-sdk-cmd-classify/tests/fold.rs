//! How per-item results fold into the run's report.
//!
//! These live outside the crate because they exercise only what it publishes,
//! and because the source file has a size limit that a thorough test module
//! eats on its own.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "a test that cannot fail loudly is not a test"
)]

use typesafe_sdk_cmd_classify::{Answered, Classified, Item};
use typesafe_sdk_cmd_kit::Usage;

fn shaky(names: &[&str]) -> Answered {
    let mut row = row(Some("m"), None);
    row.item.uncertain = !names.is_empty();
    row.shaky = names.iter().map(|n| (*n).to_owned()).collect();
    row
}

/// The per-question count is what the scalar cannot say: three rows shaky
/// on `urgency` and one on `category` is a question to reword, not four
/// rows to re-read.
#[test]
fn uncertainty_is_counted_per_question_not_only_per_row() {
    let report = Classified::of(
        "m",
        vec![
            shaky(&["urgency"]),
            shaky(&["urgency", "category"]),
            shaky(&["urgency"]),
            shaky(&[]),
        ],
    );
    assert_eq!(report.uncertain, 3, "three rows had something shaky");
    assert_eq!(report.uncertain_by_question.get("urgency"), Some(&3));
    assert_eq!(report.uncertain_by_question.get("category"), Some(&1));
}

/// A clean run says nothing rather than saying zero for every question.
#[test]
fn a_confident_run_reports_no_question_counts() {
    let report = Classified::of("m", vec![shaky(&[]), shaky(&[])]);
    assert_eq!(report.uncertain, 0);
    assert!(report.uncertain_by_question.is_empty());
}

fn row(model: Option<&str>, error: Option<&str>) -> Answered {
    Answered {
        item: Item {
            item: "x".to_owned(),
            answers: serde_json::Value::Null,
            uncertain: false,
            error: error.map(str::to_owned),
        },
        usage: Usage::default(),
        model: model.map(str::to_owned),
        shaky: Vec::new(),
    }
}

/// The reported model is the one the service answered with, not the alias
/// that was configured. Three distinct strings, so neither wrong source
/// can pass by accident.
#[test]
fn the_resolved_model_is_reported_not_the_configured_one() {
    let report = Classified::of("jev-configured", vec![row(Some("jev-1.13.0"), None)]);
    assert_eq!(report.model, "jev-1.13.0");
    assert!(report.models.is_none(), "one version is not a roll");
}

/// Nothing answered, so no response named a model and the configured
/// default is all there is to report.
#[test]
fn the_configured_model_is_the_fallback_when_nothing_answered() {
    let report = Classified::of("jev-configured", vec![row(None, Some("boom"))]);
    assert_eq!(report.model, "jev-configured");
}

/// A long run can straddle a deployment; reporting only the first version
/// would mislabel everything after the roll.
#[test]
fn a_run_that_straddles_a_deployment_names_every_version() {
    let report = Classified::of(
        "jev-configured",
        vec![row(Some("jev-1.13.0"), None), row(Some("jev-1.14.0"), None)],
    );
    assert_eq!(
        report.models.as_deref(),
        Some(["jev-1.13.0".to_owned(), "jev-1.14.0".to_owned()].as_slice())
    );
}

/// A run that answered nothing did not work, whatever it printed.
#[test]
fn exit_code_distinguishes_total_failure_from_partial() {
    let all_ok = Classified::of("m", vec![row(Some("m"), None), row(Some("m"), None)]);
    let some = Classified::of("m", vec![row(Some("m"), None), row(None, Some("boom"))]);
    let none = Classified::of("m", vec![row(None, Some("boom"))]);
    assert_eq!(all_ok.exit_code(), None, "a clean run exits zero");
    assert_eq!(some.exit_code(), Some(2), "partial failure is its own code");
    assert_eq!(
        none.exit_code(),
        Some(1),
        "nothing answered is a failed run"
    );
}

fn item(uncertain: bool, error: Option<&str>) -> Item {
    Item {
        item: "x".to_owned(),
        answers: serde_json::Value::Null,
        uncertain,
        error: error.map(str::to_owned),
    }
}

fn paid(item: Item, usage: Usage) -> Answered {
    let model = item.error.is_none().then(|| "jev-1".to_owned());
    Answered {
        item,
        usage,
        model,
        shaky: Vec::new(),
    }
}

fn spent(input: u64, output: u64) -> Usage {
    Usage {
        input_tokens: input,
        output_tokens: output,
    }
}

/// The reported total is the sum over the batch, not one item's and not
/// the last one's — the distinction only shows up past two items.
#[test]
fn usage_totals_every_item() {
    let report = Classified::of(
        "jev-1",
        vec![
            paid(item(false, None), spent(10, 1)),
            paid(item(false, None), spent(200, 20)),
            paid(item(false, None), spent(3000, 300)),
        ],
    );
    assert_eq!(report.usage.input_tokens, 3210);
    assert_eq!(report.usage.output_tokens, 321);
}

/// An item that failed was never answered, so it adds nothing to the bill
/// while still being counted as a row.
#[test]
fn a_failed_item_costs_nothing_and_still_counts() {
    let report = Classified::of(
        "jev-1",
        vec![
            paid(item(false, None), spent(7, 2)),
            paid(item(true, Some("boom")), Usage::default()),
        ],
    );
    assert_eq!(report.usage.input_tokens, 7);
    assert_eq!(report.failed, 1);
    assert_eq!(report.uncertain, 1);
    assert_eq!(report.items.len(), 2);
}

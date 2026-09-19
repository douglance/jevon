//! What an eval reports.
//!
//! A list of measurements rather than one, because wording is the largest lever
//! there is and the real operation is comparing several phrasings against the
//! same items. A single question is a list of one.

use schemars::JsonSchema;
use serde::Serialize;
use typesafe_sdk_cmd_kit::Usage;

/// How one cut-off would behave as a gate.
#[derive(Serialize, JsonSchema)]
pub struct Cut {
    /// The cut-off applied.
    pub threshold: f64,
    /// How many items it flags.
    pub flagged: usize,
    /// The share of flagged items that really are positive.
    pub precision: f64,
    /// The share of all positives it catches.
    pub recall: f64,
}

/// One question, measured.
#[derive(Serialize, JsonSchema)]
pub struct Measured {
    /// The question as it was asked.
    pub question: String,

    /// The chance a positive scores above a negative. Yes/no questions only.
    ///
    /// The number to trust, because it is the one that survives a rewording.
    /// 0.5 is a coin flip; below 0.5 means the question is answering backwards,
    /// which is a different problem from answering badly.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auc: Option<f64>,
    /// The share of items whose label is positive. A gate whose precision does
    /// not beat this is not a gate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_rate: Option<f64>,
    /// How each cut-off would behave, so a threshold is chosen against real
    /// numbers rather than inherited from another phrasing.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub cuts: Vec<Cut>,

    /// The share of answers that matched the label. Choice questions only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accuracy: Option<f64>,
    /// The same, over answers the model was confident about.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accuracy_when_confident: Option<f64>,
    /// What share of answers cleared the confidence threshold.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confident_share: Option<f64>,

    /// How far the answers sat from the middle. Needs no labels.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spread: Option<f64>,
    /// The share of answers that committed to neither side. Needs no labels.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub undecided: Option<f64>,

    /// What is wrong with this measurement, in the words a reader needs.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

/// What a run measured, best question first.
#[derive(Serialize, JsonSchema)]
pub struct Evaluated {
    /// The resolved model version that answered.
    pub model: String,
    /// How many labelled items were judged.
    pub items: usize,
    /// How many could not be answered at all.
    pub failed: usize,
    /// How many items carry a positive label.
    ///
    /// Reported next to `auc` because the number is only as good as the smaller
    /// of the two sides, and a reader should not have to remember that.
    pub positives: usize,
    /// How many carry a negative one.
    pub negatives: usize,
    /// Every question asked, ranked by how well it ordered the items.
    pub questions: Vec<Measured>,
    /// Tokens the eval spent.
    pub usage: Usage,
}

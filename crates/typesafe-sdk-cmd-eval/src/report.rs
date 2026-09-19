//! What an eval reports.
//!
//! Two shapes, because the two question kinds fail differently. A yes/no
//! question fails by not ordering; a choice question fails by picking wrong.

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

/// What a run measured.
#[derive(Serialize, JsonSchema)]
pub struct Evaluated {
    /// The resolved model version that answered.
    pub model: String,
    /// How many labelled items were judged.
    pub items: usize,
    /// How many could not be answered at all.
    pub failed: usize,
    /// `noul` or `choice`.
    pub kind: String,

    /// The chance a positive scores above a negative. Yes/no questions only.
    ///
    /// This is the number to trust, because it is the one that survives a
    /// rewording. 0.5 is a coin flip; below 0.5 means the question is answering
    /// backwards.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auc: Option<f64>,
    /// The share of items whose label is positive, for comparison against
    /// precision — a gate no better than this is not a gate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_rate: Option<f64>,
    /// How each cut-off would behave, so a threshold is chosen against real
    /// numbers rather than inherited from another question.
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

    /// Tokens the eval spent.
    pub usage: Usage,
}

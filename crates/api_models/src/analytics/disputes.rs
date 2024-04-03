use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

use super::{NameDescription, TimeRange};
use crate::enums::DisputeStage;

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    strum::Display,
    strum::EnumIter,
    strum::AsRefStr,
)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum DisputeMetrics {
    DisputesChallenged,
    DisputesWon,
    DisputesLost,
    TotalDispute,
    TotalAmountDisputed,
    TotalDisputeLostAmount,
}

#[derive(
    Debug,
    serde::Serialize,
    serde::Deserialize,
    strum::AsRefStr,
    PartialEq,
    PartialOrd,
    Eq,
    Ord,
    strum::Display,
    strum::EnumIter,
    Clone,
    Copy,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum DisputeDimensions {
    // Do not change the order of these enums
    // Consult the Dashboard FE folks since these also affects the order of metrics on FE
    Connector,
    DisputeStage,
}

impl From<DisputeDimensions> for NameDescription {
    fn from(value: DisputeDimensions) -> Self {
        Self {
            name: value.to_string(),
            desc: String::new(),
        }
    }
}

impl From<DisputeMetrics> for NameDescription {
    fn from(value: DisputeMetrics) -> Self {
        Self {
            name: value.to_string(),
            desc: String::new(),
        }
    }
}

#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
pub struct DisputeFilters {
    #[serde(default)]
    pub dispute_stage: Vec<DisputeStage>,
    pub connector: Vec<String>,
}

#[derive(Debug, serde::Serialize, Eq)]
pub struct DisputeMetricsBucketIdentifier {
    pub dispute_stage: Option<DisputeStage>,
    pub connector: Option<String>,
    #[serde(rename = "time_range")]
    pub time_bucket: TimeRange,
    #[serde(rename = "time_bucket")]
    #[serde(with = "common_utils::custom_serde::iso8601custom")]
    pub start_time: time::PrimitiveDateTime,
}

impl Hash for DisputeMetricsBucketIdentifier {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.dispute_stage.hash(state);
        self.connector.hash(state);
        self.time_bucket.hash(state);
    }
}
impl PartialEq for DisputeMetricsBucketIdentifier {
    fn eq(&self, other: &Self) -> bool {
        let mut left = DefaultHasher::new();
        self.hash(&mut left);
        let mut right = DefaultHasher::new();
        other.hash(&mut right);
        left.finish() == right.finish()
    }
}

impl DisputeMetricsBucketIdentifier {
    pub fn new(
        dispute_stage: Option<DisputeStage>,
        connector: Option<String>,
        normalized_time_range: TimeRange,
    ) -> Self {
        Self {
            dispute_stage,
            connector,
            time_bucket: normalized_time_range,
            start_time: normalized_time_range.start_time,
        }
    }
}

#[derive(Debug, serde::Serialize)]
pub struct DisputeMetricsBucketValue {
    pub disputes_challenged: Option<u64>,
    pub disputes_won: Option<u64>,
    pub disputes_lost: Option<u64>,
    pub total_amount_disputed: Option<u64>,
    pub total_dispute_lost_amount: Option<u64>,
    pub total_dispute: Option<u64>,
}
#[derive(Debug, serde::Serialize)]
pub struct DisputeMetricsBucketResponse {
    #[serde(flatten)]
    pub values: DisputeMetricsBucketValue,
    #[serde(flatten)]
    pub dimensions: DisputeMetricsBucketIdentifier,
}

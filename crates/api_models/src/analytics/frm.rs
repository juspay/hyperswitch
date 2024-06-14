use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

use common_enums::enums::FraudCheckStatus;

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Eq,
    PartialEq,
    serde::Serialize,
    serde::Deserialize,
    strum::Display,
    strum::EnumString,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum FrmTransactionType {
    #[default]
    PreFrm,
    PostFrm,
}

use super::{NameDescription, TimeRange};
#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
pub struct FrmFilters {
    #[serde(default)]
    pub frm_status: Vec<FraudCheckStatus>,
    #[serde(default)]
    pub frm_name: Vec<String>,
    #[serde(default)]
    pub frm_transaction_type: Vec<FrmTransactionType>,
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
pub enum FrmDimensions {
    FrmStatus,
    FrmName,
    FrmTransactionType,
}

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
pub enum FrmMetrics {
    FrmTriggeredAttempts,
    FrmBlockedRate,
}

pub mod metric_behaviour {
    pub struct FrmTriggeredAttempts;
    pub struct FrmBlockRate;
}

impl From<FrmMetrics> for NameDescription {
    fn from(value: FrmMetrics) -> Self {
        Self {
            name: value.to_string(),
            desc: String::new(),
        }
    }
}

impl From<FrmDimensions> for NameDescription {
    fn from(value: FrmDimensions) -> Self {
        Self {
            name: value.to_string(),
            desc: String::new(),
        }
    }
}

#[derive(Debug, serde::Serialize, Eq)]
pub struct FrmMetricsBucketIdentifier {
    pub frm_status: Option<String>,
    pub frm_name: Option<String>,
    pub frm_transaction_type: Option<String>,
    #[serde(rename = "time_range")]
    pub time_bucket: TimeRange,
    #[serde(rename = "time_bucket")]
    #[serde(with = "common_utils::custom_serde::iso8601custom")]
    pub start_time: time::PrimitiveDateTime,
}

impl Hash for FrmMetricsBucketIdentifier {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.frm_status.hash(state);
        self.frm_name.hash(state);
        self.frm_transaction_type.hash(state);
        self.time_bucket.hash(state);
    }
}

impl PartialEq for FrmMetricsBucketIdentifier {
    fn eq(&self, other: &Self) -> bool {
        let mut left = DefaultHasher::new();
        self.hash(&mut left);
        let mut right = DefaultHasher::new();
        other.hash(&mut right);
        left.finish() == right.finish()
    }
}

impl FrmMetricsBucketIdentifier {
    pub fn new(
        frm_status: Option<String>,
        frm_name: Option<String>,
        frm_transaction_type: Option<String>,
        normalized_time_range: TimeRange,
    ) -> Self {
        Self {
            frm_status,
            frm_name,
            frm_transaction_type,
            time_bucket: normalized_time_range,
            start_time: normalized_time_range.start_time,
        }
    }
}

#[derive(Debug, serde::Serialize)]
pub struct FrmMetricsBucketValue {
    pub frm_triggered_attempts: Option<u64>,
    pub frm_blocked_rate: Option<f64>,
}

#[derive(Debug, serde::Serialize)]
pub struct FrmMetricsBucketResponse {
    #[serde(flatten)]
    pub values: FrmMetricsBucketValue,
    #[serde(flatten)]
    pub dimensions: FrmMetricsBucketIdentifier,
}

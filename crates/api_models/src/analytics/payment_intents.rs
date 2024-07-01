use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

use super::{NameDescription, TimeRange};
use crate::enums::{Currency, IntentStatus};

#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
pub struct PaymentIntentFilters {
    #[serde(default)]
    pub status: Vec<IntentStatus>,
    pub currency: Vec<Currency>,
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
pub enum PaymentIntentDimensions {
    #[strum(serialize = "status")]
    #[serde(rename = "status")]
    PaymentIntentStatus,
    Currency,
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
pub enum PaymentIntentMetrics {
    SuccessfulSmartRetries,
    TotalSmartRetries,
    SmartRetriedAmount,
    PaymentIntentCount,
}

#[derive(Debug, Default, serde::Serialize)]
pub struct ErrorResult {
    pub reason: String,
    pub count: i64,
    pub percentage: f64,
}

pub mod metric_behaviour {
    pub struct SuccessfulSmartRetries;
    pub struct TotalSmartRetries;
    pub struct SmartRetriedAmount;
    pub struct PaymentIntentCount;
}

impl From<PaymentIntentMetrics> for NameDescription {
    fn from(value: PaymentIntentMetrics) -> Self {
        Self {
            name: value.to_string(),
            desc: String::new(),
        }
    }
}

impl From<PaymentIntentDimensions> for NameDescription {
    fn from(value: PaymentIntentDimensions) -> Self {
        Self {
            name: value.to_string(),
            desc: String::new(),
        }
    }
}

#[derive(Debug, serde::Serialize, Eq)]
pub struct PaymentIntentMetricsBucketIdentifier {
    pub status: Option<IntentStatus>,
    pub currency: Option<Currency>,
    #[serde(rename = "time_range")]
    pub time_bucket: TimeRange,
    #[serde(rename = "time_bucket")]
    #[serde(with = "common_utils::custom_serde::iso8601custom")]
    pub start_time: time::PrimitiveDateTime,
}

impl PaymentIntentMetricsBucketIdentifier {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        status: Option<IntentStatus>,
        currency: Option<Currency>,
        normalized_time_range: TimeRange,
    ) -> Self {
        Self {
            status,
            currency,
            time_bucket: normalized_time_range,
            start_time: normalized_time_range.start_time,
        }
    }
}

impl Hash for PaymentIntentMetricsBucketIdentifier {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.status.map(|i| i.to_string()).hash(state);
        self.currency.hash(state);
        self.time_bucket.hash(state);
    }
}

impl PartialEq for PaymentIntentMetricsBucketIdentifier {
    fn eq(&self, other: &Self) -> bool {
        let mut left = DefaultHasher::new();
        self.hash(&mut left);
        let mut right = DefaultHasher::new();
        other.hash(&mut right);
        left.finish() == right.finish()
    }
}

#[derive(Debug, serde::Serialize)]
pub struct PaymentIntentMetricsBucketValue {
    pub successful_smart_retries: Option<u64>,
    pub total_smart_retries: Option<u64>,
    pub smart_retried_amount: Option<u64>,
    pub payment_intent_count: Option<u64>,
}

#[derive(Debug, serde::Serialize)]
pub struct MetricsBucketResponse {
    #[serde(flatten)]
    pub values: PaymentIntentMetricsBucketValue,
    #[serde(flatten)]
    pub dimensions: PaymentIntentMetricsBucketIdentifier,
}

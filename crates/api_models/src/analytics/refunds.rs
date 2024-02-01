use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

use crate::{enums::Currency, refunds::RefundStatus};

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
// TODO RefundType api_models_oss need to mapped to storage_model
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum RefundType {
    InstantRefund,
    #[default]
    RegularRefund,
    RetryRefund,
}

use super::{NameDescription, TimeRange};
#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
pub struct RefundFilters {
    #[serde(default)]
    pub currency: Vec<Currency>,
    #[serde(default)]
    pub refund_status: Vec<RefundStatus>,
    #[serde(default)]
    pub connector: Vec<String>,
    #[serde(default)]
    pub refund_type: Vec<RefundType>,
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
pub enum RefundDimensions {
    Currency,
    RefundStatus,
    Connector,
    RefundType,
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
pub enum RefundMetrics {
    RefundSuccessRate,
    RefundCount,
    RefundSuccessCount,
    RefundProcessedAmount,
}

pub mod metric_behaviour {
    pub struct RefundSuccessRate;
    pub struct RefundCount;
    pub struct RefundSuccessCount;
    pub struct RefundProcessedAmount;
}

impl From<RefundMetrics> for NameDescription {
        /// Creates a new instance of Self by converting a RefundMetrics value into a new instance with the same name and an empty description.
    fn from(value: RefundMetrics) -> Self {
        Self {
            name: value.to_string(),
            desc: String::new(),
        }
    }
}

impl From<RefundDimensions> for NameDescription {
        /// This method creates a new instance of the current type by taking a `RefundDimensions` value and using its `to_string` method to set the `name` field, while setting the `desc` field to an empty string.
    fn from(value: RefundDimensions) -> Self {
        Self {
            name: value.to_string(),
            desc: String::new(),
        }
    }
}

#[derive(Debug, serde::Serialize, Eq)]
pub struct RefundMetricsBucketIdentifier {
    pub currency: Option<Currency>,
    pub refund_status: Option<String>,
    pub connector: Option<String>,

    pub refund_type: Option<String>,
    #[serde(rename = "time_range")]
    pub time_bucket: TimeRange,
    #[serde(rename = "time_bucket")]
    #[serde(with = "common_utils::custom_serde::iso8601custom")]
    pub start_time: time::PrimitiveDateTime,
}

impl Hash for RefundMetricsBucketIdentifier {
        /// Hashes the fields of the struct using the given Hasher instance.
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.currency.hash(state);
        self.refund_status.hash(state);
        self.connector.hash(state);
        self.refund_type.hash(state);
        self.time_bucket.hash(state);
    }
}
impl PartialEq for RefundMetricsBucketIdentifier {
        /// Compares two instances of the same type for equality based on their hash values.
    fn eq(&self, other: &Self) -> bool {
        let mut left = DefaultHasher::new();
        self.hash(&mut left);
        let mut right = DefaultHasher::new();
        other.hash(&mut right);
        left.finish() == right.finish()
    }
}

impl RefundMetricsBucketIdentifier {
        /// Creates a new instance of the struct, initializing the fields with the provided values.
    pub fn new(
        currency: Option<Currency>,
        refund_status: Option<String>,
        connector: Option<String>,
        refund_type: Option<String>,
        normalized_time_range: TimeRange,
    ) -> Self {
        Self {
            currency,
            refund_status,
            connector,
            refund_type,
            time_bucket: normalized_time_range,
            start_time: normalized_time_range.start_time,
        }
    }
}
#[derive(Debug, serde::Serialize)]
pub struct RefundMetricsBucketValue {
    pub refund_success_rate: Option<f64>,
    pub refund_count: Option<u64>,
    pub refund_success_count: Option<u64>,
    pub refund_processed_amount: Option<u64>,
}
#[derive(Debug, serde::Serialize)]
pub struct RefundMetricsBucketResponse {
    #[serde(flatten)]
    pub values: RefundMetricsBucketValue,
    #[serde(flatten)]
    pub dimensions: RefundMetricsBucketIdentifier,
}

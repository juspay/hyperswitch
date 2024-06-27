use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

use super::NameDescription;

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
pub enum ActivePaymentsMetrics {
    ActivePayments,
}

pub mod metric_behaviour {
    pub struct ActivePayments;
}

impl From<ActivePaymentsMetrics> for NameDescription {
    fn from(value: ActivePaymentsMetrics) -> Self {
        Self {
            name: value.to_string(),
            desc: String::new(),
        }
    }
}

#[derive(Debug, serde::Serialize, Eq)]
pub struct ActivePaymentsMetricsBucketIdentifier {
    pub time_bucket: Option<String>,
}

impl ActivePaymentsMetricsBucketIdentifier {
    pub fn new(time_bucket: Option<String>) -> Self {
        Self { time_bucket }
    }
}

impl Hash for ActivePaymentsMetricsBucketIdentifier {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.time_bucket.hash(state);
    }
}

impl PartialEq for ActivePaymentsMetricsBucketIdentifier {
    fn eq(&self, other: &Self) -> bool {
        let mut left = DefaultHasher::new();
        self.hash(&mut left);
        let mut right = DefaultHasher::new();
        other.hash(&mut right);
        left.finish() == right.finish()
    }
}

#[derive(Debug, serde::Serialize)]
pub struct ActivePaymentsMetricsBucketValue {
    pub active_payments: Option<u64>,
}

#[derive(Debug, serde::Serialize)]
pub struct MetricsBucketResponse {
    #[serde(flatten)]
    pub values: ActivePaymentsMetricsBucketValue,
    #[serde(flatten)]
    pub dimensions: ActivePaymentsMetricsBucketIdentifier,
}

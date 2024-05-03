use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

use super::{NameDescription, TimeRange};
use crate::enums::{
    AttemptStatus, AuthenticationType, Connector, Currency, PaymentMethod, PaymentMethodType,
};

#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
pub struct PaymentFilters {
    #[serde(default)]
    pub currency: Vec<Currency>,
    #[serde(default)]
    pub status: Vec<AttemptStatus>,
    #[serde(default)]
    pub connector: Vec<Connector>,
    #[serde(default)]
    pub auth_type: Vec<AuthenticationType>,
    #[serde(default)]
    pub payment_method: Vec<PaymentMethod>,
    #[serde(default)]
    pub payment_method_type: Vec<PaymentMethodType>,
    #[serde(default)]
    pub merchant_id: Vec<String>,
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
pub enum PaymentDimensions {
    // Do not change the order of these enums
    // Consult the Dashboard FE folks since these also affects the order of metrics on FE
    Connector,
    PaymentMethod,
    PaymentMethodType,
    Currency,
    #[strum(serialize = "authentication_type")]
    #[serde(rename = "authentication_type")]
    AuthType,
    #[strum(serialize = "status")]
    #[serde(rename = "status")]
    PaymentStatus,
    #[strum(serialize = "merchant_id")]
    #[serde(rename = "merchant_id")]
    MerchantId,
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
pub enum PaymentMetrics {
    PaymentSuccessRate,
    PaymentCount,
    PaymentSuccessCount,
    PaymentProcessedAmount,
    AvgTicketSize,
    RetriesCount,
    ConnectorSuccessRate,
}

#[derive(Debug, Default, serde::Serialize)]
pub struct ErrorResult {
    pub reason: String,
    pub count: i64,
    pub percentage: f64,
}

#[derive(
    Clone,
    Copy,
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
pub enum PaymentDistributions {
    #[strum(serialize = "error_message")]
    PaymentErrorMessage,
}

pub mod metric_behaviour {
    pub struct PaymentSuccessRate;
    pub struct PaymentCount;
    pub struct PaymentSuccessCount;
    pub struct PaymentProcessedAmount;
    pub struct AvgTicketSize;
}

impl From<PaymentMetrics> for NameDescription {
    fn from(value: PaymentMetrics) -> Self {
        Self {
            name: value.to_string(),
            desc: String::new(),
        }
    }
}

impl From<PaymentDimensions> for NameDescription {
    fn from(value: PaymentDimensions) -> Self {
        Self {
            name: value.to_string(),
            desc: String::new(),
        }
    }
}

#[derive(Debug, serde::Serialize, Eq)]
pub struct PaymentMetricsBucketIdentifier {
    pub currency: Option<Currency>,
    pub status: Option<AttemptStatus>,
    pub connector: Option<String>,
    #[serde(rename = "authentication_type")]
    pub auth_type: Option<AuthenticationType>,
    pub payment_method: Option<String>,
    pub payment_method_type: Option<String>,
    #[serde(rename = "time_range")]
    pub time_bucket: TimeRange,
    // Coz FE sucks
    #[serde(rename = "time_bucket")]
    #[serde(with = "common_utils::custom_serde::iso8601custom")]
    pub start_time: time::PrimitiveDateTime,
}

impl PaymentMetricsBucketIdentifier {
    pub fn new(
        currency: Option<Currency>,
        status: Option<AttemptStatus>,
        connector: Option<String>,
        auth_type: Option<AuthenticationType>,
        payment_method: Option<String>,
        payment_method_type: Option<String>,
        normalized_time_range: TimeRange,
    ) -> Self {
        Self {
            currency,
            status,
            connector,
            auth_type,
            payment_method,
            payment_method_type,
            time_bucket: normalized_time_range,
            start_time: normalized_time_range.start_time,
        }
    }
}

impl Hash for PaymentMetricsBucketIdentifier {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.currency.hash(state);
        self.status.map(|i| i.to_string()).hash(state);
        self.connector.hash(state);
        self.auth_type.map(|i| i.to_string()).hash(state);
        self.payment_method.hash(state);
        self.payment_method_type.hash(state);
        self.time_bucket.hash(state);
    }
}

impl PartialEq for PaymentMetricsBucketIdentifier {
    fn eq(&self, other: &Self) -> bool {
        let mut left = DefaultHasher::new();
        self.hash(&mut left);
        let mut right = DefaultHasher::new();
        other.hash(&mut right);
        left.finish() == right.finish()
    }
}

#[derive(Debug, serde::Serialize)]
pub struct PaymentMetricsBucketValue {
    pub payment_success_rate: Option<f64>,
    pub payment_count: Option<u64>,
    pub payment_success_count: Option<u64>,
    pub payment_processed_amount: Option<u64>,
    pub avg_ticket_size: Option<f64>,
    pub payment_error_message: Option<Vec<ErrorResult>>,
    pub retries_count: Option<u64>,
    pub retries_amount_processed: Option<u64>,
    pub connector_success_rate: Option<f64>,
}

#[derive(Debug, serde::Serialize)]
pub struct MetricsBucketResponse {
    #[serde(flatten)]
    pub values: PaymentMetricsBucketValue,
    #[serde(flatten)]
    pub dimensions: PaymentMetricsBucketIdentifier,
}

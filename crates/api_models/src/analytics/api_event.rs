use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

use super::{NameDescription, TimeRange};
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct ApiLogsRequest {
    #[serde(flatten)]
    pub query_param: QueryType,
    pub api_name_filter: Option<Vec<String>>,
}

pub enum FilterType {
    ApiCountFilter,
    LatencyFilter,
    StatusCodeFilter,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(tag = "type")]
pub enum QueryType {
    Payment {
        payment_id: String,
    },
    Refund {
        payment_id: String,
        refund_id: String,
    },
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
pub enum ApiEventDimensions {
    // Do not change the order of these enums
    // Consult the Dashboard FE folks since these also affects the order of metrics on FE
    StatusCode,
    FlowType,
    ApiFlow,
}

impl From<ApiEventDimensions> for NameDescription {
        /// Converts an ApiEventDimensions struct into a new instance of the current struct.
    ///
    /// # Arguments
    ///
    /// * `value` - The ApiEventDimensions struct to convert
    ///
    /// # Returns
    ///
    /// A new instance of the current struct with the name field set to the string representation of the value parameter and the desc field set to an empty string.
    ///
    fn from(value: ApiEventDimensions) -> Self {
        Self {
            name: value.to_string(),
            desc: String::new(),
        }
    }
}
#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
pub struct ApiEventFilters {
    pub status_code: Vec<u64>,
    pub flow_type: Vec<String>,
    pub api_flow: Vec<String>,
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
pub enum ApiEventMetrics {
    Latency,
    ApiCount,
    StatusCodeCount,
}

impl From<ApiEventMetrics> for NameDescription {
        /// Converts an ApiEventMetrics struct into a new instance of Self, initializing the name field with the string representation of the given value and the desc field with an empty string.
    fn from(value: ApiEventMetrics) -> Self {
        Self {
            name: value.to_string(),
            desc: String::new(),
        }
    }
}

#[derive(Debug, serde::Serialize, Eq)]
pub struct ApiEventMetricsBucketIdentifier {
    #[serde(rename = "time_range")]
    pub time_bucket: TimeRange,
    // Coz FE sucks
    #[serde(rename = "time_bucket")]
    #[serde(with = "common_utils::custom_serde::iso8601custom")]
    pub start_time: time::PrimitiveDateTime,
}

impl ApiEventMetricsBucketIdentifier {
        /// Creates a new instance of Self with the provided normalized_time_range. 
    /// 
    /// # Arguments
    /// 
    /// * `normalized_time_range` - The normalized time range used to initialize the time_bucket and start_time.
    /// 
    /// # Returns
    /// 
    /// A new instance of Self with the time_bucket set to `normalized_time_range` and start_time set to `normalized_time_range.start_time`.
    pub fn new(normalized_time_range: TimeRange) -> Self {
        Self {
            time_bucket: normalized_time_range,
            start_time: normalized_time_range.start_time,
        }
    }
}

impl Hash for ApiEventMetricsBucketIdentifier {
        /// Hashes the current instance by hashing its time_bucket field and updating the provided Hasher state.
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.time_bucket.hash(state);
    }
}

impl PartialEq for ApiEventMetricsBucketIdentifier {
        /// Compares the current object with another object for equality by hashing both objects
    /// using DefaultHasher and comparing the resulting hash values.
    fn eq(&self, other: &Self) -> bool {
        let mut left = DefaultHasher::new();
        self.hash(&mut left);
        let mut right = DefaultHasher::new();
        other.hash(&mut right);
        left.finish() == right.finish()
    }
}

#[derive(Debug, serde::Serialize)]
pub struct ApiEventMetricsBucketValue {
    pub latency: Option<u64>,
    pub api_count: Option<u64>,
    pub status_code_count: Option<u64>,
}

#[derive(Debug, serde::Serialize)]
pub struct ApiMetricsBucketResponse {
    #[serde(flatten)]
    pub values: ApiEventMetricsBucketValue,
    #[serde(flatten)]
    pub dimensions: ApiEventMetricsBucketIdentifier,
}

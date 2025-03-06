use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

use common_enums::{AuthenticationConnectors, AuthenticationStatus, TransactionStatus};

use super::{NameDescription, TimeRange};

#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
pub struct AuthEventFilters {
    #[serde(default)]
    pub authentication_status: Vec<AuthenticationStatus>,
    #[serde(default)]
    pub trans_status: Vec<TransactionStatus>,
    #[serde(default)]
    pub error_message: Vec<String>,
    #[serde(default)]
    pub authentication_connector: Vec<AuthenticationConnectors>,
    #[serde(default)]
    pub message_version: Vec<String>,
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
pub enum AuthEventDimensions {
    AuthenticationStatus,
    #[strum(serialize = "trans_status")]
    #[serde(rename = "trans_status")]
    TransactionStatus,
    ErrorMessage,
    AuthenticationConnector,
    MessageVersion,
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
pub enum AuthEventMetrics {
    AuthenticationCount,
    AuthenticationAttemptCount,
    AuthenticationSuccessCount,
    ChallengeFlowCount,
    FrictionlessFlowCount,
    FrictionlessSuccessCount,
    ChallengeAttemptCount,
    ChallengeSuccessCount,
    AuthenticationErrorMessage,
    AuthenticationFunnel,
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
pub enum AuthEventFlows {
    IncomingWebhookReceive,
    PaymentsExternalAuthentication,
}

pub mod metric_behaviour {
    pub struct AuthenticationCount;
    pub struct AuthenticationAttemptCount;
    pub struct AuthenticationSuccessCount;
    pub struct ChallengeFlowCount;
    pub struct FrictionlessFlowCount;
    pub struct FrictionlessSuccessCount;
    pub struct ChallengeAttemptCount;
    pub struct ChallengeSuccessCount;
    pub struct AuthenticationErrorMessage;
}

impl From<AuthEventMetrics> for NameDescription {
    fn from(value: AuthEventMetrics) -> Self {
        Self {
            name: value.to_string(),
            desc: String::new(),
        }
    }
}

impl From<AuthEventDimensions> for NameDescription {
    fn from(value: AuthEventDimensions) -> Self {
        Self {
            name: value.to_string(),
            desc: String::new(),
        }
    }
}

#[derive(Debug, serde::Serialize, Eq)]
pub struct AuthEventMetricsBucketIdentifier {
    pub authentication_status: Option<AuthenticationStatus>,
    pub trans_status: Option<TransactionStatus>,
    pub error_message: Option<String>,
    pub authentication_connector: Option<AuthenticationConnectors>,
    pub message_version: Option<String>,
    #[serde(rename = "time_range")]
    pub time_bucket: TimeRange,
    #[serde(rename = "time_bucket")]
    #[serde(with = "common_utils::custom_serde::iso8601custom")]
    pub start_time: time::PrimitiveDateTime,
}

impl AuthEventMetricsBucketIdentifier {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        authentication_status: Option<AuthenticationStatus>,
        trans_status: Option<TransactionStatus>,
        error_message: Option<String>,
        authentication_connector: Option<AuthenticationConnectors>,
        message_version: Option<String>,
        normalized_time_range: TimeRange,
    ) -> Self {
        Self {
            authentication_status,
            trans_status,
            error_message,
            authentication_connector,
            message_version,
            time_bucket: normalized_time_range,
            start_time: normalized_time_range.start_time,
        }
    }
}

impl Hash for AuthEventMetricsBucketIdentifier {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.authentication_status.hash(state);
        self.trans_status.hash(state);
        self.authentication_connector.hash(state);
        self.message_version.hash(state);
        self.error_message.hash(state);
        self.time_bucket.hash(state);
    }
}

impl PartialEq for AuthEventMetricsBucketIdentifier {
    fn eq(&self, other: &Self) -> bool {
        let mut left = DefaultHasher::new();
        self.hash(&mut left);
        let mut right = DefaultHasher::new();
        other.hash(&mut right);
        left.finish() == right.finish()
    }
}

#[derive(Debug, serde::Serialize)]
pub struct AuthEventMetricsBucketValue {
    pub authentication_count: Option<u64>,
    pub authentication_attempt_count: Option<u64>,
    pub authentication_success_count: Option<u64>,
    pub challenge_flow_count: Option<u64>,
    pub challenge_attempt_count: Option<u64>,
    pub challenge_success_count: Option<u64>,
    pub frictionless_flow_count: Option<u64>,
    pub frictionless_success_count: Option<u64>,
    pub error_message_count: Option<u64>,
    pub authentication_funnel: Option<u64>,
}

#[derive(Debug, serde::Serialize)]
pub struct MetricsBucketResponse {
    #[serde(flatten)]
    pub values: AuthEventMetricsBucketValue,
    #[serde(flatten)]
    pub dimensions: AuthEventMetricsBucketIdentifier,
}

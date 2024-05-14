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
pub enum AuthEventMetrics {
    ThreeDsSdkCount,
    AuthenticationAttemptCount,
    AuthenticationSuccessCount,
    ChallengeFlowCount,
    FrictionlessFlowCount,
    ChallengeAttemptCount,
    ChallengeSuccessCount,
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
    PostAuthentication,
}

pub mod metric_behaviour {
    pub struct ThreeDsSdkCount;
    pub struct AuthenticationAttemptCount;
    pub struct AuthenticationSuccessCount;
    pub struct ChallengeFlowCount;
    pub struct FrictionlessFlowCount;
    pub struct ChallengeAttemptCount;
    pub struct ChallengeSuccessCount;
}

impl From<AuthEventMetrics> for NameDescription {
    fn from(value: AuthEventMetrics) -> Self {
        Self {
            name: value.to_string(),
            desc: String::new(),
        }
    }
}

#[derive(Debug, serde::Serialize, Eq)]
pub struct AuthEventMetricsBucketIdentifier {
    pub time_bucket: Option<String>,
}

impl AuthEventMetricsBucketIdentifier {
    pub fn new(time_bucket: Option<String>) -> Self {
        Self { time_bucket }
    }
}

impl Hash for AuthEventMetricsBucketIdentifier {
    fn hash<H: Hasher>(&self, state: &mut H) {
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
    pub three_ds_sdk_count: Option<u64>,
    pub authentication_attempt_count: Option<u64>,
    pub authentication_success_count: Option<u64>,
    pub challenge_flow_count: Option<u64>,
    pub challenge_attempt_count: Option<u64>,
    pub challenge_success_count: Option<u64>,
    pub frictionless_flow_count: Option<u64>,
}

#[derive(Debug, serde::Serialize)]
pub struct MetricsBucketResponse {
    #[serde(flatten)]
    pub values: AuthEventMetricsBucketValue,
    #[serde(flatten)]
    pub dimensions: AuthEventMetricsBucketIdentifier,
}

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "version", rename_all = "snake_case")]
pub enum AlertRuleConfig {
    V1(AlertRuleConfigV1),
}

impl AlertRuleConfig {
    pub fn inner(&self) -> &AlertRuleConfigV1 {
        match self {
            Self::V1(config) => config,
        }
    }

    pub fn into_inner(self) -> AlertRuleConfigV1 {
        match self {
            Self::V1(config) => config,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AlertRuleConfigV1 {
    pub query: AlertRuleQuery,
    pub frequency: u32,
    pub severities: BTreeMap<Severity, SeverityRuleV1>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "version", rename_all = "snake_case")]
pub enum AlertRuleQuery {
    V1(AlertRuleQueryV1),
}

impl AlertRuleQuery {
    pub fn inner(&self) -> &AlertRuleQueryV1 {
        match self {
            Self::V1(query) => query,
        }
    }

    pub fn into_inner(self) -> AlertRuleQueryV1 {
        match self {
            Self::V1(query) => query,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "data_source", rename_all = "snake_case")]
pub enum AlertRuleQueryV1 {
    Cloudwatch(CloudwatchQueryV1),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CloudwatchQueryV1 {
    pub region: String,
    pub namespace: String,
    pub metric_name: String,
    pub dimensions: BTreeMap<String, String>,
    pub statistic: CloudwatchStatistic,
    pub period: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, strum::Display)]
pub enum CloudwatchStatistic {
    Average,
    Maximum,
    Minimum,
    Sum,
}

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, strum::Display,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum Severity {
    Sev1,
    Sev2,
    Sev3,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SeverityRuleV1 {
    pub operator: ComparisonOperator,
    pub threshold: f64,
    pub description: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, strum::Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ComparisonOperator {
    Gt,
    Gte,
    Lt,
    Lte,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AlertRuleConfigurationCreateRequest {
    pub name: String,
    pub config: AlertRuleConfig,
    pub is_enabled: Option<bool>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AlertRuleConfigurationUpdateRequest {
    pub config: Option<AlertRuleConfig>,
    pub is_enabled: Option<bool>,
}

#[derive(Clone, Debug, Serialize)]
pub struct AlertRuleConfigurationResponse {
    pub id: String,
    pub name: String,
    pub is_enabled: bool,
    pub config: AlertRuleConfig,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub next_run_at: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub last_updated_at: PrimitiveDateTime,
}

#[derive(Clone, Debug, Serialize)]
pub struct AlertRuleConfigurationListResponse {
    pub rule_configurations: Vec<AlertRuleConfigurationResponse>,
    pub unreadable_ids: Vec<String>,
}

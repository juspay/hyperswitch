use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CloudwatchQueryV1 {
    pub region: String,
    pub namespace: String,
    pub metric_name: String,
    #[serde(default)]
    pub dimensions: BTreeMap<String, String>,
    pub statistic: CloudwatchStatistic,
    pub period: u32,
    #[serde(default = "default_end_offset_seconds")]
    pub end_offset_seconds: u32,
}

const DEFAULT_END_OFFSET_SECONDS: u32 = 180;

fn default_end_offset_seconds() -> u32 {
    DEFAULT_END_OFFSET_SECONDS
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AlertRuleThresholdsV1 {
    pub severities: BTreeMap<Severity, SeverityThresholdV1>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SeverityThresholdV1 {
    pub threshold: f64,
    pub comparison_operator: ComparisonOperator,
    pub evaluation_periods: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub datapoints_to_alarm: Option<u32>,
    #[serde(default)]
    pub treat_missing_data: TreatMissingData,
    pub description: String,
}

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum AlertDatasource {
    Cloudwatch,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, strum::Display)]
pub enum ComparisonOperator {
    GreaterThanThreshold,
    GreaterThanOrEqualToThreshold,
    LessThanThreshold,
    LessThanOrEqualToThreshold,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, strum::Display)]
#[serde(rename_all = "camelCase")]
#[strum(serialize_all = "camelCase")]
pub enum TreatMissingData {
    Breaching,
    NotBreaching,
    #[default]
    Missing,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AlertRuleConfigurationCreateRequest {
    pub id: String,
    pub name: String,
    pub classification: String,
    pub datasource: AlertDatasource,
    pub query: CloudwatchQueryV1,
    pub thresholds: AlertRuleThresholdsV1,
    pub interval_seconds: u32,
    pub is_enabled: Option<bool>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AlertRuleConfigurationUpdateRequest {
    pub name: Option<String>,
    pub classification: Option<String>,
    pub query: Option<CloudwatchQueryV1>,
    pub thresholds: Option<AlertRuleThresholdsV1>,
    pub interval_seconds: Option<u32>,
    pub is_enabled: Option<bool>,
}

#[derive(Clone, Debug, Serialize)]
pub struct AlertRuleConfigurationResponse {
    pub id: String,
    pub name: String,
    pub classification: String,
    pub datasource: AlertDatasource,
    pub query: CloudwatchQueryV1,
    pub thresholds: AlertRuleThresholdsV1,
    pub interval_seconds: u32,
    pub is_enabled: bool,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub next_run_at: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub last_evaluated_at: Option<PrimitiveDateTime>,
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

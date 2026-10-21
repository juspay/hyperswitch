use std::collections::BTreeMap;

use common_utils::impl_to_sql_from_sql_json;
use diesel::{sql_types::Jsonb, AsExpression, FromSqlRow};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromSqlRow, AsExpression)]
#[diesel(sql_type = Jsonb)]
#[serde(tag = "version", rename_all = "snake_case")]
pub enum AlertRuleConfig {
    V1(AlertRuleConfigV1),
}

impl_to_sql_from_sql_json!(AlertRuleConfig);

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
    pub classification: String,
    pub interval_seconds: u32,
    pub query: AlertQuery,
    pub thresholds: AlertThresholdsV1,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "datasource", rename_all = "snake_case")]
pub enum AlertQuery {
    Cloudwatch(CloudwatchQueryV1),
}

impl AlertQuery {
    pub fn datasource(&self) -> AlertDatasource {
        match self {
            Self::Cloudwatch(_) => AlertDatasource::Cloudwatch,
        }
    }
}

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
pub struct AlertThresholdsV1 {
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
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    strum::Display,
    strum::EnumString,
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
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, strum::Display,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum Severity {
    Critical,
    Warning,
    Info,
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

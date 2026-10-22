use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AlertRuleConfig {
    pub interval_seconds: u32,
    pub query: AlertQuery,
    pub thresholds: AlertThresholds,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "datasource", rename_all = "snake_case")]
pub enum AlertQuery {
    GrafanaCloudwatch(GrafanaCloudwatchQuery),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GrafanaCloudwatchQuery {
    pub region: String,
    pub namespace: String,
    pub metric_name: String,
    #[serde(default)]
    pub dimensions: BTreeMap<String, String>,
    pub statistic: CloudwatchStatistic,
    pub period: u32,
    #[serde(default = "DEFAULT_END_OFFSET_SECONDS")]
    pub end_offset_seconds: u32,
}

const DEFAULT_END_OFFSET_SECONDS: fn() -> u32 = || 180;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AlertThresholds {
    pub severities: BTreeMap<Severity, SeverityThreshold>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SeverityThreshold {
    pub threshold: f64,
    pub comparison_operator: ComparisonOperator,
    pub description: String,
    pub destination: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, strum::Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum MetricSource {
    GrafanaCloudwatch,
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
    #[strum(to_string = ">")]
    GreaterThanThreshold,
    #[strum(to_string = ">=")]
    GreaterThanOrEqualToThreshold,
    #[strum(to_string = "<")]
    LessThanThreshold,
    #[strum(to_string = "<=")]
    LessThanOrEqualToThreshold,
}

impl ComparisonOperator {
    pub fn breaches(self, value: f64, threshold: f64) -> bool {
        match self {
            Self::GreaterThanThreshold => value > threshold,
            Self::GreaterThanOrEqualToThreshold => value >= threshold,
            Self::LessThanThreshold => value < threshold,
            Self::LessThanOrEqualToThreshold => value <= threshold,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comparison_display_preserves_wire_names_and_threshold_boundaries() {
        for (operator, symbol, wire, above, equal) in [
            (
                ComparisonOperator::GreaterThanThreshold,
                ">",
                "GreaterThanThreshold",
                true,
                false,
            ),
            (
                ComparisonOperator::GreaterThanOrEqualToThreshold,
                ">=",
                "GreaterThanOrEqualToThreshold",
                true,
                true,
            ),
            (
                ComparisonOperator::LessThanThreshold,
                "<",
                "LessThanThreshold",
                false,
                false,
            ),
            (
                ComparisonOperator::LessThanOrEqualToThreshold,
                "<=",
                "LessThanOrEqualToThreshold",
                false,
                true,
            ),
        ] {
            assert_eq!(operator.to_string(), symbol);
            assert_eq!(
                serde_json::to_string(&operator).unwrap(),
                format!("\"{wire}\"")
            );
            assert_eq!(operator.breaches(11.0, 10.0), above);
            assert_eq!(operator.breaches(10.0, 10.0), equal);
        }
    }

    #[test]
    fn cloudwatch_end_offset_defaults_to_three_minutes() {
        let query: AlertQuery = serde_json::from_value(serde_json::json!({
            "datasource": "grafana_cloudwatch",
            "region": "us-east-1",
            "namespace": "AWS/ApplicationELB",
            "metric_name": "RequestCount",
            "statistic": "Sum",
            "period": 60
        }))
        .unwrap();
        let AlertQuery::GrafanaCloudwatch(query) = query;
        assert_eq!(query.end_offset_seconds, 180);
    }
}

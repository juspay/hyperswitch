use std::{collections::BTreeMap, fmt};

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

impl AlertThresholds {
    pub fn error_destination(&self) -> Option<&str> {
        self.severities
            .first_key_value()
            .map(|(_, threshold)| threshold.destination.as_str())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SeverityThreshold {
    pub threshold: f64,
    pub comparison_operator: ComparisonOperator,
    pub description: String,
    pub destination: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MetricSource {
    GrafanaCloudwatch,
}

impl fmt::Display for MetricSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::GrafanaCloudwatch => "grafana_cloudwatch",
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CloudwatchStatistic {
    Average,
    Maximum,
    Minimum,
    Sum,
}

impl fmt::Display for CloudwatchStatistic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Average => "Average",
            Self::Maximum => "Maximum",
            Self::Minimum => "Minimum",
            Self::Sum => "Sum",
        })
    }
}

/// Ordered highest priority first for threshold and error-destination selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Critical,
    Warning,
    Info,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Critical => "critical",
            Self::Warning => "warning",
            Self::Info => "info",
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComparisonOperator {
    GreaterThanThreshold,
    GreaterThanOrEqualToThreshold,
    LessThanThreshold,
    LessThanOrEqualToThreshold,
}

impl fmt::Display for ComparisonOperator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::GreaterThanThreshold => ">",
            Self::GreaterThanOrEqualToThreshold => ">=",
            Self::LessThanThreshold => "<",
            Self::LessThanOrEqualToThreshold => "<=",
        })
    }
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
    fn severity_order_is_highest_priority_first() {
        let mut severities = [Severity::Info, Severity::Critical, Severity::Warning];
        severities.sort_unstable();
        assert_eq!(
            severities,
            [Severity::Critical, Severity::Warning, Severity::Info]
        );
    }

    #[test]
    fn error_destination_uses_highest_priority_threshold() {
        let threshold = |destination: &str| SeverityThreshold {
            threshold: 1.0,
            comparison_operator: ComparisonOperator::GreaterThanThreshold,
            description: String::new(),
            destination: destination.to_owned(),
        };
        let thresholds = AlertThresholds {
            severities: BTreeMap::from([
                (Severity::Warning, threshold("warning")),
                (Severity::Critical, threshold("critical")),
            ]),
        };
        assert_eq!(thresholds.error_destination(), Some("critical"));
        assert_eq!(
            AlertThresholds {
                severities: BTreeMap::new(),
            }
            .error_destination(),
            None
        );
    }

    #[test]
    fn comparison_display_preserves_wire_names_and_threshold_boundaries() {
        assert_eq!(
            MetricSource::GrafanaCloudwatch.to_string(),
            "grafana_cloudwatch"
        );
        for (statistic, name) in [
            (CloudwatchStatistic::Average, "Average"),
            (CloudwatchStatistic::Maximum, "Maximum"),
            (CloudwatchStatistic::Minimum, "Minimum"),
            (CloudwatchStatistic::Sum, "Sum"),
        ] {
            assert_eq!(statistic.to_string(), name);
        }
        for (severity, name) in [
            (Severity::Critical, "critical"),
            (Severity::Warning, "warning"),
            (Severity::Info, "info"),
        ] {
            assert_eq!(severity.to_string(), name);
        }
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

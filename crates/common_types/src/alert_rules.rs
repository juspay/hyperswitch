pub mod stored;

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
    GrafanaLoki(GrafanaLokiQuery),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GrafanaLokiQuery {
    pub expression: String,
    pub mode: LokiQueryMode,
    pub range_seconds: u32,
    #[serde(default)]
    pub step_seconds: Option<u32>,
    pub reducer: LokiReducer,
    #[serde(default)]
    pub delivery_enabled: bool,
    #[serde(default)]
    pub pending_seconds: u32,
    #[serde(default)]
    pub labels: BTreeMap<String, String>,
    #[serde(default)]
    pub fields: BTreeMap<String, String>,
}

impl GrafanaLokiQuery {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.expression.trim().is_empty()
            || !(1..=86400).contains(&self.range_seconds)
            || self.step_seconds == Some(0)
            || (self.mode == LokiQueryMode::Range && self.step_seconds.is_none())
        {
            return Err(
                "Loki requires a nonblank expression, bounded range and positive range step",
            );
        }
        if self.delivery_enabled && self.pending_seconds != 0 {
            return Err("Loki delivery with pending timers requires persistent instance state; use preview only");
        }
        if self.fields.keys().any(|key| {
            !matches!(
                key.as_str(),
                "error"
                    | "application"
                    | "environment"
                    | "path"
                    | "method"
                    | "flow"
                    | "connector"
                    | "file"
                    | "request_id"
                    | "event_time"
                    | "error_code"
            )
        }) {
            return Err("unsupported application message field mapping");
        }
        if self
            .labels
            .iter()
            .chain(self.fields.iter())
            .any(|(k, v)| k.trim().is_empty() || v.trim().is_empty())
        {
            return Err("Loki static labels and field mappings must not be blank");
        }
        self.expanded_expression()?;
        Ok(())
    }

    pub fn expanded_expression(&self) -> Result<String, &'static str> {
        let mut result = String::new();
        let mut remaining = self.expression.as_str();
        while let Some(index) = remaining.find("$__") {
            result.push_str(&remaining[..index]);
            let token = &remaining[index..];
            let end = token
                .char_indices()
                .skip(1)
                .find(|(_, c)| !c.is_ascii_alphanumeric() && *c != '_')
                .map_or(token.len(), |(index, _)| index);
            let seconds = match &token[..end] {
                "$__range" => self.range_seconds,
                "$__auto" if self.mode == LokiQueryMode::Instant => self.range_seconds,
                "$__auto" | "$__interval" => self
                    .step_seconds
                    .ok_or("Loki macro requires an explicit step")?,
                _ => return Err("unsupported Loki macro"),
            };
            if seconds == 0 {
                return Err("Loki macro duration must be positive");
            }
            result.push_str(&format!("{seconds}s"));
            remaining = &token[end..];
        }
        result.push_str(remaining);
        Ok(result)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LokiQueryMode {
    Instant,
    Range,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LokiReducer {
    Last,
    Count,
    Sum,
    Minimum,
    Maximum,
    Mean,
}

impl AlertQuery {
    pub fn metric_source(&self) -> MetricSource {
        match self {
            Self::GrafanaCloudwatch(_) => MetricSource::GrafanaCloudwatch,
            Self::GrafanaLoki(_) => MetricSource::GrafanaLoki,
        }
    }
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
    GrafanaLoki,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, strum::Display)]
pub enum CloudwatchStatistic {
    Average,
    Maximum,
    Minimum,
    Sum,
}

/// Ordered highest priority first for threshold and error-destination selection.
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
    fn loki_macros_are_exact_and_do_not_change_regex_anchors() {
        let mut query = GrafanaLokiQuery {
            expression: "count_over_time({app=\"test\"} |~ `failed$` [$__auto]) + count_over_time({app=\"test\"}[$__range])".to_owned(),
            mode: LokiQueryMode::Instant,
            range_seconds: 300,
            step_seconds: Some(10),
            reducer: LokiReducer::Last,
            delivery_enabled: false,
            pending_seconds: 0,
            labels: BTreeMap::new(),
            fields: BTreeMap::new(),
        };
        assert_eq!(query.expanded_expression().unwrap(), "count_over_time({app=\"test\"} |~ `failed$` [300s]) + count_over_time({app=\"test\"}[300s])");
        query.mode = LokiQueryMode::Range;
        assert!(query.expanded_expression().unwrap().contains("[10s]"));
        query.expression = "$__range_ms".to_owned();
        assert!(query.expanded_expression().is_err());
        query.expression = "$__interval".to_owned();
        query.step_seconds = None;
        assert!(query.expanded_expression().is_err());
    }

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
        let AlertQuery::GrafanaCloudwatch(query) = query else {
            panic!("expected CloudWatch query");
        };
        assert_eq!(query.end_offset_seconds, 180);
    }
}

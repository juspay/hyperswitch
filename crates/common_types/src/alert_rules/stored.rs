//! The typed columns of `alert_rules`: the rule's `status` and its versioned
//! `alerts_configuration`, with the checks a configuration passes before it is stored.
//!
//! The configuration, its thresholds and each datasource's query carry their own `version` label,
//! so one part can change format without the others. Writers always write the newest version;
//! readers accept every version still listed here. A new optional field (with a default) needs no
//! new version; a new datasource is a new [`AlertQuery`] variant with its own versions.
//!
//! The evaluator works on [`AlertRuleConfig`], the current shape without versions. A stored
//! configuration converts into it, so a new version changes that conversion, not the evaluator.

use std::{collections::BTreeMap, str::FromStr};

use diesel::{
    sql_types::{Jsonb, Text},
    AsExpression, FromSqlRow,
};
use serde::{Deserialize, Serialize};

use super::{AlertRuleConfig, CloudwatchStatistic, ComparisonOperator, Severity};

/// The evaluator runs on a one-minute tick, so a rule is checked every whole number of minutes, at
/// least once a day.
const MIN_INTERVAL_SECONDS: u32 = 60;
const MAX_INTERVAL_SECONDS: u32 = 86_400;
/// CloudWatch's own limit on dimensions per metric.
const MAX_DIMENSIONS: usize = 30;
/// The evaluator's limit on how far back the newest data point may be cut off.
const MAX_END_OFFSET_SECONDS: u32 = 3_600;

/// `alert_rules.status`: whether the evaluator checks a rule. Stored as text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize, AsExpression, FromSqlRow)]
#[diesel(sql_type = Text)]
#[serde(rename_all = "snake_case")]
pub enum AlertRuleStatus {
    /// Checked by the evaluator.
    Active,
    /// Kept, but not checked.
    Inactive,
}

impl AlertRuleStatus {
    /// The value stored in `alert_rules.status`.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Inactive => "inactive",
        }
    }
}

impl FromStr for AlertRuleStatus {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "active" => Ok(Self::Active),
            "inactive" => Ok(Self::Inactive),
            other => Err(format!(
                "status `{other}` is not one of `active`, `inactive`"
            )),
        }
    }
}

impl diesel::serialize::ToSql<Text, diesel::pg::Pg> for AlertRuleStatus {
    fn to_sql<'b>(
        &'b self,
        out: &mut diesel::serialize::Output<'b, '_, diesel::pg::Pg>,
    ) -> diesel::serialize::Result {
        <str as diesel::serialize::ToSql<Text, diesel::pg::Pg>>::to_sql(self.as_str(), out)
    }
}

impl diesel::deserialize::FromSql<Text, diesel::pg::Pg> for AlertRuleStatus {
    fn from_sql(bytes: diesel::pg::PgValue<'_>) -> diesel::deserialize::Result<Self> {
        let value =
            <String as diesel::deserialize::FromSql<Text, diesel::pg::Pg>>::from_sql(bytes)?;
        Ok(value.parse()?)
    }
}

/// `alert_rules.alerts_configuration`: how a rule is checked.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize, AsExpression, FromSqlRow)]
#[diesel(sql_type = Jsonb)]
#[serde(tag = "version", rename_all = "snake_case")]
pub enum AlertConfiguration {
    /// The first version.
    V1(AlertConfigurationV1),
}

common_utils::impl_to_sql_from_sql_json!(AlertConfiguration);

/// Version 1 of the configuration.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AlertConfigurationV1 {
    /// How often the rule is checked, in seconds. Not the size of a data point; that is the
    /// query's `period`.
    pub interval_seconds: u32,
    /// The query, versioned per datasource, so it can change format without a new configuration
    /// version.
    pub query: AlertQuery,
    /// The versioned thresholds, so they can change format without a new configuration version.
    pub thresholds: AlertThresholds,
}

/// What to fetch, chosen by `datasource`. Each datasource versions its own query.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(tag = "datasource", rename_all = "snake_case")]
pub enum AlertQuery {
    /// A CloudWatch metric.
    Cloudwatch(CloudwatchQuery),
    Loki(LokiQuery),
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(tag = "version", rename_all = "snake_case")]
pub enum LokiQuery {
    V1(super::GrafanaLokiQuery),
}

/// A CloudWatch metric query.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(tag = "version", rename_all = "snake_case")]
pub enum CloudwatchQuery {
    /// The first version.
    V1(CloudwatchQueryV1),
}

/// Version 1 of the CloudWatch query, with the fields and meaning of a CloudWatch alarm's metric.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CloudwatchQueryV1 {
    /// AWS region the metric lives in, e.g. `us-east-1`.
    pub region: String,
    /// Metric namespace, e.g. `AWS/RDS`.
    pub namespace: String,
    /// Metric name, e.g. `CPUUtilization`.
    pub metric_name: String,
    /// Exact CloudWatch dimension names (case-sensitive) and values.
    #[serde(default)]
    pub dimensions: BTreeMap<String, String>,
    /// How the data points in each period are combined.
    pub statistic: CloudwatchStatistic,
    /// Size of each data point, in seconds. Not how often the rule is checked.
    pub period: u32,
    /// How many seconds before the check the newest data point is cut off, so data CloudWatch is
    /// still collecting is not evaluated. Left out, 180.
    #[serde(default = "end_offset_default")]
    pub end_offset_seconds: u32,
}

fn end_offset_default() -> u32 {
    180
}

/// When the query's result counts as breaching.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(tag = "version", rename_all = "snake_case")]
pub enum AlertThresholds {
    /// The first version.
    V1(AlertThresholdsV1),
}

/// Version 1 of the thresholds.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AlertThresholdsV1 {
    /// One tier per severity, each checked on its own. At least one.
    pub severities: BTreeMap<Severity, ThresholdV1>,
}

/// One tier: when it breaches, and where its alerts go.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ThresholdV1 {
    /// The value the newest data point is compared with.
    pub threshold: f64,
    /// How the newest data point is compared with `threshold`.
    pub comparison_operator: ComparisonOperator,
    /// The message for this tier when it fires.
    pub description: String,
    /// The chat destination this tier's alerts go to: an id configured under `chat.destinations`.
    pub destination: String,
}

/// Empty, or only whitespace.
fn blank(value: &str) -> bool {
    value.trim().is_empty()
}

impl AlertConfiguration {
    /// Check the configuration. The error names the failing field, e.g.
    /// `alerts_configuration.thresholds.severities.critical.destination`.
    pub fn validate(&self) -> Result<(), String> {
        match self {
            Self::V1(configuration) => configuration.validate("alerts_configuration"),
        }
    }
}

impl AlertConfigurationV1 {
    fn validate(&self, path: &str) -> Result<(), String> {
        let interval = self.interval_seconds;
        if !(MIN_INTERVAL_SECONDS..=MAX_INTERVAL_SECONDS).contains(&interval) || interval % 60 != 0
        {
            return Err(format!(
                "{path}.interval_seconds must be a whole number of minutes from \
                 {MIN_INTERVAL_SECONDS} to {MAX_INTERVAL_SECONDS}, not {interval}"
            ));
        }

        self.query.validate(&format!("{path}.query"))?;
        self.thresholds.validate(&format!("{path}.thresholds"))
    }
}

impl AlertQuery {
    fn validate(&self, path: &str) -> Result<(), String> {
        match self {
            Self::Cloudwatch(CloudwatchQuery::V1(query)) => query.validate(path),
            Self::Loki(LokiQuery::V1(query)) => query
                .validate()
                .map_err(|reason| format!("{path}: {reason}")),
        }
    }
}

impl CloudwatchQueryV1 {
    fn validate(&self, path: &str) -> Result<(), String> {
        if blank(&self.region) {
            return Err(format!("{path}.region must not be empty"));
        }
        if blank(&self.namespace) {
            return Err(format!("{path}.namespace must not be empty"));
        }
        if blank(&self.metric_name) {
            return Err(format!("{path}.metric_name must not be empty"));
        }

        if self.dimensions.len() > MAX_DIMENSIONS {
            return Err(format!(
                "{path}.dimensions: at most {MAX_DIMENSIONS} are allowed, not {}",
                self.dimensions.len()
            ));
        }
        if self
            .dimensions
            .iter()
            .any(|(name, value)| blank(name) || blank(value))
        {
            return Err(format!(
                "{path}.dimensions: names and values must not be empty"
            ));
        }

        // What CloudWatch accepts: high-resolution periods, or whole minutes up to a day.
        let period = self.period;
        let high_resolution = matches!(period, 1 | 5 | 10 | 20 | 30);
        let standard = period > 0 && period % 60 == 0 && period <= 86_400;
        if !(high_resolution || standard) {
            return Err(format!(
                "{path}.period must be 1, 5, 10, 20, 30 or a multiple of 60 up to 86400, not \
                 {period}"
            ));
        }

        if self.end_offset_seconds > MAX_END_OFFSET_SECONDS {
            return Err(format!(
                "{path}.end_offset_seconds must be at most {MAX_END_OFFSET_SECONDS}, not {}",
                self.end_offset_seconds
            ));
        }

        Ok(())
    }
}

impl AlertThresholds {
    fn validate(&self, path: &str) -> Result<(), String> {
        match self {
            Self::V1(thresholds) => thresholds.validate(path),
        }
    }
}

impl AlertThresholdsV1 {
    fn validate(&self, path: &str) -> Result<(), String> {
        if self.severities.is_empty() {
            return Err(format!("{path}.severities must have at least one entry"));
        }

        self.severities
            .iter()
            .try_for_each(|(severity, threshold)| {
                threshold.validate(&format!("{path}.severities.{severity}"))
            })
    }
}

impl ThresholdV1 {
    fn validate(&self, path: &str) -> Result<(), String> {
        if !self.threshold.is_finite() {
            return Err(format!("{path}.threshold must be a finite number"));
        }
        if blank(&self.description) {
            return Err(format!("{path}.description must not be empty"));
        }
        if blank(&self.destination) {
            return Err(format!("{path}.destination must not be empty"));
        }
        Ok(())
    }
}

/// The evaluator's shape. Every stored field is carried over, so a configuration that passed
/// [`AlertConfiguration::validate`] also passes the evaluator's own checks.
impl From<AlertConfiguration> for AlertRuleConfig {
    fn from(configuration: AlertConfiguration) -> Self {
        let AlertConfiguration::V1(configuration) = configuration;

        let query = match configuration.query {
            AlertQuery::Loki(LokiQuery::V1(query)) => super::AlertQuery::GrafanaLoki(query),
            AlertQuery::Cloudwatch(CloudwatchQuery::V1(query)) => {
                super::AlertQuery::GrafanaCloudwatch(super::GrafanaCloudwatchQuery {
                    region: query.region,
                    namespace: query.namespace,
                    metric_name: query.metric_name,
                    dimensions: query.dimensions,
                    statistic: query.statistic,
                    period: query.period,
                    end_offset_seconds: query.end_offset_seconds,
                })
            }
        };

        let AlertThresholds::V1(thresholds) = configuration.thresholds;
        let severities = thresholds
            .severities
            .into_iter()
            .map(|(severity, threshold)| {
                (
                    severity,
                    super::SeverityThreshold {
                        threshold: threshold.threshold,
                        comparison_operator: threshold.comparison_operator,
                        description: threshold.description,
                        destination: threshold.destination,
                    },
                )
            })
            .collect();

        Self {
            interval_seconds: configuration.interval_seconds,
            query,
            thresholds: super::AlertThresholds { severities },
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]
mod tests {
    use serde_json::{json, Value};

    use super::*;

    /// A complete `alerts_configuration`, as stored.
    fn stored() -> Value {
        json!({
            "version": "v1",
            "interval_seconds": 60,
            "query": {
                "datasource": "cloudwatch",
                "version": "v1",
                "region": "us-east-1",
                "namespace": "AWS/RDS",
                "metric_name": "CPUUtilization",
                "dimensions": { "DBClusterIdentifier": "hyperswitchdb-cluster", "Role": "WRITER" },
                "statistic": "Average",
                "period": 60,
                "end_offset_seconds": 180
            },
            "thresholds": {
                "version": "v1",
                "severities": {
                    "critical": {
                        "threshold": 80.0,
                        "comparison_operator": "GreaterThanOrEqualToThreshold",
                        "description": "RDS writer CPU above 80%.",
                        "destination": "infra_oncall"
                    },
                    "warning": {
                        "threshold": 70.0,
                        "comparison_operator": "GreaterThanOrEqualToThreshold",
                        "description": "RDS writer CPU above 70%.",
                        "destination": "infra_alerts"
                    },
                    "info": {
                        "threshold": 60.0,
                        "comparison_operator": "GreaterThanOrEqualToThreshold",
                        "description": "RDS writer CPU above 60%.",
                        "destination": "infra_alerts"
                    }
                }
            }
        })
    }

    fn configuration(value: Value) -> AlertConfiguration {
        serde_json::from_value(value).unwrap()
    }

    fn parse_error(value: Value) -> String {
        serde_json::from_value::<AlertConfiguration>(value)
            .unwrap_err()
            .to_string()
    }

    fn set(mut value: Value, pointer: &str, replacement: Value) -> Value {
        *value.pointer_mut(pointer).unwrap() = replacement;
        value
    }

    fn remove(mut value: Value, pointer: &str, field: &str) -> Value {
        value
            .pointer_mut(pointer)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .remove(field);
        value
    }

    #[test]
    fn a_stored_configuration_reads_and_writes_back_unchanged() {
        let parsed = configuration(stored());
        assert_eq!(serde_json::to_value(&parsed).unwrap(), stored());
        assert_eq!(parsed.validate(), Ok(()));

        let AlertConfiguration::V1(v1) = parsed;
        let AlertQuery::Cloudwatch(CloudwatchQuery::V1(query)) = v1.query else {
            panic!("Expected CloudWatch fixture");
        };
        assert_eq!(query.statistic, CloudwatchStatistic::Average);
        let AlertThresholds::V1(thresholds) = v1.thresholds;
        assert_eq!(
            thresholds.severities.keys().copied().collect::<Vec<_>>(),
            [Severity::Critical, Severity::Warning, Severity::Info]
        );
    }

    #[test]
    fn loki_stored_query_roundtrip_preserves_context_and_delivery_guard() {
        let mut value = stored();
        value["query"] = json!({"datasource":"loki","version":"v1","expression":"vector(1)","mode":"instant","range_seconds":300,"reducer":"last","fields":{"error":"extra_error"},"labels":{"environment":"sandbox"},"delivery_enabled":false,"pending_seconds":10800});
        let parsed = configuration(value.clone());
        assert_eq!(parsed.validate(), Ok(()));
        let runtime = AlertRuleConfig::from(parsed.clone());
        assert_eq!(
            runtime.query.metric_source(),
            super::super::MetricSource::GrafanaLoki
        );
        let written = serde_json::to_value(parsed).unwrap();
        assert_eq!(written["query"]["fields"], value["query"]["fields"]);
        assert_eq!(written["query"]["pending_seconds"], 10800);
        value["query"]["delivery_enabled"] = json!(true);
        assert!(configuration(value.clone())
            .validate()
            .unwrap_err()
            .contains("pending"));
        value["query"]["pending_seconds"] = json!(0);
        value["query"]["fields"] = json!({"unrestricted_raw_payload":"payload"});
        assert!(configuration(value).validate().is_err());
    }

    #[test]
    fn left_out_fields_take_their_defaults_and_are_written_explicitly() {
        let minimal = remove(
            remove(stored(), "/query", "dimensions"),
            "/query",
            "end_offset_seconds",
        );
        let written = serde_json::to_value(configuration(minimal)).unwrap();

        assert_eq!(written["query"]["dimensions"], json!({}));
        assert_eq!(written["query"]["end_offset_seconds"], 180);
    }

    #[test]
    fn every_part_needs_a_version_this_code_knows() {
        for pointer in ["", "/query", "/thresholds"] {
            let missing = remove(stored(), pointer, "version");
            assert!(
                parse_error(missing).contains("version"),
                "missing version at `{pointer}`"
            );

            let unknown = set(stored(), &format!("{pointer}/version"), json!("v2"));
            assert!(
                parse_error(unknown).contains("v2"),
                "unknown version at `{pointer}`"
            );
        }
    }

    #[test]
    fn unknown_fields_are_rejected_at_every_level() {
        for pointer in [
            "",
            "/query",
            "/thresholds",
            "/thresholds/severities/critical",
        ] {
            let mut value = stored();
            value
                .pointer_mut(pointer)
                .unwrap()
                .as_object_mut()
                .unwrap()
                .insert("threshhold".to_owned(), json!(1));
            assert!(
                parse_error(value).contains("unknown field `threshhold`"),
                "unknown field at `{pointer}`"
            );
        }
    }

    #[test]
    fn unknown_names_are_rejected() {
        for (pointer, value) in [
            ("/query/datasource", json!("victoria_metrics")),
            ("/query/statistic", json!("p99")),
            (
                "/thresholds/severities/critical/comparison_operator",
                json!("GreaterThanUpperThreshold"),
            ),
        ] {
            assert!(
                serde_json::from_value::<AlertConfiguration>(set(stored(), pointer, value))
                    .is_err()
            );
        }

        let renamed = {
            let mut value = stored();
            let critical = value["thresholds"]["severities"]["critical"].clone();
            value["thresholds"]["severities"] = json!({ "sev1": critical });
            value
        };
        assert!(parse_error(renamed).contains("sev1"));
    }

    #[test]
    fn every_check_names_its_field() {
        let critical = "/thresholds/severities/critical";
        let cases = [
            (
                "alerts_configuration.interval_seconds",
                set(stored(), "/interval_seconds", json!(90)),
            ),
            (
                "alerts_configuration.interval_seconds",
                set(stored(), "/interval_seconds", json!(0)),
            ),
            (
                "alerts_configuration.interval_seconds",
                set(stored(), "/interval_seconds", json!(86_460)),
            ),
            (
                "alerts_configuration.query.region",
                set(stored(), "/query/region", json!(" ")),
            ),
            (
                "alerts_configuration.query.namespace",
                set(stored(), "/query/namespace", json!("")),
            ),
            (
                "alerts_configuration.query.metric_name",
                set(stored(), "/query/metric_name", json!("")),
            ),
            (
                "alerts_configuration.query.dimensions",
                set(stored(), "/query/dimensions", json!({ "Role": " " })),
            ),
            (
                "alerts_configuration.query.dimensions",
                set(
                    stored(),
                    "/query/dimensions",
                    Value::Object(
                        (0..=MAX_DIMENSIONS)
                            .map(|index| (format!("Dimension{index}"), json!("value")))
                            .collect(),
                    ),
                ),
            ),
            (
                "alerts_configuration.query.period",
                set(stored(), "/query/period", json!(61)),
            ),
            (
                "alerts_configuration.query.period",
                set(stored(), "/query/period", json!(0)),
            ),
            (
                "alerts_configuration.query.period",
                set(stored(), "/query/period", json!(86_460)),
            ),
            (
                "alerts_configuration.query.end_offset_seconds",
                set(stored(), "/query/end_offset_seconds", json!(3_601)),
            ),
            (
                "alerts_configuration.thresholds.severities",
                set(stored(), "/thresholds/severities", json!({})),
            ),
            (
                "alerts_configuration.thresholds.severities.critical.description",
                set(stored(), &format!("{critical}/description"), json!(" ")),
            ),
            (
                "alerts_configuration.thresholds.severities.critical.destination",
                set(stored(), &format!("{critical}/destination"), json!("")),
            ),
        ];

        for (field, value) in cases {
            let reason = configuration(value).validate().unwrap_err();
            assert!(reason.starts_with(field), "expected `{field}`: {reason}");
        }

        // JSON cannot carry a non-finite number, but a configuration built in code can.
        let mut not_finite = configuration(stored());
        let AlertConfiguration::V1(v1) = &mut not_finite;
        let AlertThresholds::V1(thresholds) = &mut v1.thresholds;
        thresholds
            .severities
            .get_mut(&Severity::Critical)
            .unwrap()
            .threshold = f64::NAN;
        let reason = not_finite.validate().unwrap_err();
        assert!(reason.starts_with("alerts_configuration.thresholds.severities.critical.threshold"));
    }

    #[test]
    fn converts_to_the_evaluator_shape_with_every_field() {
        let converted = AlertRuleConfig::from(configuration(stored()));

        assert_eq!(
            serde_json::to_value(converted).unwrap(),
            json!({
                "interval_seconds": 60,
                "query": {
                    "datasource": "grafana_cloudwatch",
                    "region": "us-east-1",
                    "namespace": "AWS/RDS",
                    "metric_name": "CPUUtilization",
                    "dimensions": { "DBClusterIdentifier": "hyperswitchdb-cluster", "Role": "WRITER" },
                    "statistic": "Average",
                    "period": 60,
                    "end_offset_seconds": 180
                },
                "thresholds": {
                    "severities": {
                        "critical": {
                            "threshold": 80.0,
                            "comparison_operator": "GreaterThanOrEqualToThreshold",
                            "description": "RDS writer CPU above 80%.",
                            "destination": "infra_oncall"
                        },
                        "warning": {
                            "threshold": 70.0,
                            "comparison_operator": "GreaterThanOrEqualToThreshold",
                            "description": "RDS writer CPU above 70%.",
                            "destination": "infra_alerts"
                        },
                        "info": {
                            "threshold": 60.0,
                            "comparison_operator": "GreaterThanOrEqualToThreshold",
                            "description": "RDS writer CPU above 60%.",
                            "destination": "infra_alerts"
                        }
                    }
                }
            })
        );
    }

    #[test]
    fn statuses_read_back_from_their_stored_text_and_json() {
        for status in [AlertRuleStatus::Active, AlertRuleStatus::Inactive] {
            assert_eq!(status.as_str().parse::<AlertRuleStatus>(), Ok(status));
            assert_eq!(serde_json::to_value(status).unwrap(), status.as_str());
            assert_eq!(
                serde_json::from_value::<AlertRuleStatus>(json!(status.as_str())).unwrap(),
                status
            );
        }
        assert!("paused".parse::<AlertRuleStatus>().is_err());
    }
}

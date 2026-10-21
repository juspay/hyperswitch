use std::collections::HashSet;

use common_types::alert_rules::{AlertQuery, AlertRuleConfig, Severity};
use common_utils::fp_utils::when;
use error_stack::ResultExt;
use external_services::grafana::GrafanaConfig;
use hyperswitch_masking::Secret;
use serde::{de::Error as _, Deserialize, Deserializer};

use crate::{
    domain::evaluator::{cloudwatch, Level, RuleDefinition},
    errors::ConfigurationError,
    settings::ChatSettings,
};

#[derive(Clone, Debug, Deserialize)]
pub struct EvaluatorSettings {
    pub grafana: GrafanaConfig,
    pub datasources: EvaluatorDatasources,
    pub destinations: EvaluatorDestinations,
    #[serde(deserialize_with = "deserialize_rules")]
    rules: Vec<RuleDefinition>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct EvaluatorDatasources {
    pub cloudwatch: String,
}

impl EvaluatorDatasources {
    pub(crate) fn configured_names(&self) -> Vec<&'static str> {
        [("cloudwatch", &self.cloudwatch)]
            .into_iter()
            .filter(|(_, uid)| !uid.trim().is_empty())
            .map(|(name, _)| name)
            .collect()
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct EvaluatorDestinations {
    pub critical: String,
    pub warning: String,
    pub info: String,
}

#[derive(Debug, Deserialize)]
struct ConfiguredRule {
    id: String,
    name: String,
    config: AlertRuleConfig,
}

impl From<ConfiguredRule> for RuleDefinition {
    fn from(rule: ConfiguredRule) -> Self {
        Self {
            id: rule.id,
            name: rule.name,
            config: rule.config,
        }
    }
}

impl EvaluatorSettings {
    pub(crate) fn validate(&self, chat: &ChatSettings) -> Result<(), ConfigurationError> {
        when(
            !matches!(self.grafana.base_url.scheme(), "http" | "https"),
            || {
                Err(configuration_error(
                    None,
                    "grafana.base_url",
                    "must use the http or https scheme",
                ))
            },
        )?;

        when(self.datasources.cloudwatch.trim().is_empty(), || {
            Err(configuration_error(
                None,
                "datasources.cloudwatch",
                "must not be blank",
            ))
        })?;
        let ids = self
            .rules
            .iter()
            .map(|rule| &rule.id)
            .collect::<HashSet<_>>();
        when(ids.len() != self.rules.len(), || {
            Err(configuration_error(None, "rules.id", "must be unique"))
        })?;

        for (field, destination) in [
            ("destinations.critical", &self.destinations.critical),
            ("destinations.warning", &self.destinations.warning),
            ("destinations.info", &self.destinations.info),
        ] {
            when(!chat.destinations.contains_key(destination), || {
                Err(configuration_error(
                    None,
                    field,
                    format!("names unconfigured chat destination `{destination}`"),
                ))
            })?;
        }

        for rule in &self.rules {
            validate_rule(rule, &self.datasources)?;
        }
        Ok(())
    }

    pub(crate) fn rules(&self) -> &[RuleDefinition] {
        &self.rules
    }

    pub(crate) fn with_api_token(mut self, api_token: Secret<String>) -> Self {
        self.grafana.api_token = api_token;
        self
    }
}

impl EvaluatorDestinations {
    pub(crate) fn for_severity(&self, level: Level) -> &str {
        match level.severity() {
            Some(Severity::Critical) => &self.critical,
            Some(Severity::Warning) => &self.warning,
            Some(Severity::Info) => &self.info,
            None => "",
        }
    }
}

fn deserialize_rules<'de, D>(deserializer: D) -> Result<Vec<RuleDefinition>, D::Error>
where
    D: Deserializer<'de>,
{
    let raw = String::deserialize(deserializer)?;
    parse_rules(&raw).map_err(|error| D::Error::custom(error.current_context()))
}

fn parse_rules(raw: &str) -> error_stack::Result<Vec<RuleDefinition>, ConfigurationError> {
    serde_json::from_str::<Vec<ConfiguredRule>>(raw)
        .change_context(ConfigurationError::ConfigParsingError(
            "invalid evaluator rules JSON".to_owned(),
        ))
        .map(|rules| rules.into_iter().map(RuleDefinition::from).collect())
}

fn validate_rule(
    rule: &RuleDefinition,
    _datasources: &EvaluatorDatasources,
) -> Result<(), ConfigurationError> {
    when(rule.id.trim().is_empty(), || {
        Err(configuration_error(Some(rule), "id", "must not be blank"))
    })?;

    let config = rule.config.inner();
    when(config.interval_seconds != 60, || {
        Err(configuration_error(
            Some(rule),
            "config.interval_seconds",
            "must equal 60",
        ))
    })?;
    when(config.thresholds.severities.is_empty(), || {
        Err(configuration_error(
            Some(rule),
            "config.thresholds.severities",
            "must contain at least one severity",
        ))
    })?;

    for (severity, threshold) in &config.thresholds.severities {
        when(threshold.evaluation_periods == 0, || {
            Err(configuration_error(
                Some(rule),
                &format!("config.thresholds.severities.{severity}.evaluation_periods"),
                "must be at least 1",
            ))
        })?;
        when(
            threshold.datapoints_to_alarm.is_some_and(|datapoints| {
                datapoints == 0 || datapoints > threshold.evaluation_periods
            }),
            || {
                Err(configuration_error(
                    Some(rule),
                    &format!("config.thresholds.severities.{severity}.datapoints_to_alarm"),
                    "must be from 1 through evaluation_periods",
                ))
            },
        )?;
    }

    match &config.query {
        AlertQuery::Cloudwatch(query) => {
            let invalid = |error: cloudwatch::CloudwatchValidationError| {
                configuration_error(Some(rule), &error.field, error.requirement)
            };
            cloudwatch::validate_query(query.inner()).map_err(invalid)?;
            cloudwatch::validate_rule(rule, query.inner()).map_err(invalid)?;
        }
    }
    Ok(())
}

fn configuration_error(
    rule: Option<&RuleDefinition>,
    field: &str,
    requirement: impl Into<String>,
) -> ConfigurationError {
    let field = match rule {
        Some(rule) => format!("rule `{}` field `{field}`", rule.id),
        None => format!("field `{field}`"),
    };
    ConfigurationError::ConfigParsingError(format!(
        "invalid evaluator configuration: {field} {}",
        requirement.into()
    ))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use std::collections::HashMap;

    use common_types::alert_rules::{AlertQuery, CloudwatchQuery};
    use serde_json::{json, Value};

    use super::*;
    use crate::settings::ChatDestination;

    fn valid_json() -> String {
        json!([{
            "id": "rule-1",
            "name": "Rule 1",
            "config": {
                "version": "v1",
                "classification": "test-alerts",
                "interval_seconds": 60,
                "query": {
                    "datasource": "cloudwatch",
                    "version": "v1",
                    "region": "us-east-1",
                    "namespace": "AWS/EC2",
                    "metric_name": "CPUUtilization",
                    "dimensions": {"InstanceId": "i-0123456789abcdef0"},
                    "statistic": "Average",
                    "period": 60,
                    "end_offset_seconds": 0
                },
                "thresholds": {"severities": {"critical": {
                    "threshold": 90,
                    "comparison_operator": "GreaterThanOrEqualToThreshold",
                    "evaluation_periods": 1,
                    "datapoints_to_alarm": 1,
                    "treat_missing_data": "breaching",
                    "description": "Critical description"
                }}}
            }
        }])
        .to_string()
    }

    fn settings() -> EvaluatorSettings {
        serde_json::from_value(json!({
            "grafana": {
                "base_url": "https://grafana.example",
                "api_token": "token"
            },
            "datasources": {"cloudwatch": "cloudwatch"},
            "destinations": {
                "critical": "critical",
                "warning": "warning",
                "info": "info"
            },
            "rules": valid_json()
        }))
        .unwrap()
    }

    fn chat(ids: &[&str]) -> ChatSettings {
        ChatSettings {
            destinations: ids
                .iter()
                .map(|id| ((*id).to_owned(), ChatDestination::Log))
                .collect::<HashMap<_, _>>(),
            ..Default::default()
        }
    }

    fn mutate(pointer: &str, replacement: Value) -> EvaluatorSettings {
        let mut value: Value = serde_json::from_str(&valid_json()).unwrap();
        *value.pointer_mut(pointer).unwrap() = replacement;
        let mut settings = settings();
        settings.rules = parse_rules(&value.to_string()).unwrap();
        settings
    }

    fn validation_error(settings: &EvaluatorSettings) -> String {
        settings
            .validate(&chat(&["critical", "warning", "info"]))
            .unwrap_err()
            .to_string()
    }

    #[test]
    fn valid_rule_preserves_dimension_case() {
        let rules = parse_rules(&valid_json()).unwrap();
        let AlertQuery::Cloudwatch(CloudwatchQuery::V1(query)) = &rules[0].config.inner().query;
        assert!(query.dimensions.contains_key("InstanceId"));
        assert!(!query.dimensions.contains_key("instanceid"));
    }

    #[test]
    fn blank_region_is_rejected() {
        assert!(
            validation_error(&mutate("/0/config/query/region", json!(" ")))
                .contains("config.query.region")
        );
    }

    #[test]
    fn blank_namespace_is_rejected() {
        assert!(
            validation_error(&mutate("/0/config/query/namespace", json!(" ")))
                .contains("config.query.namespace")
        );
    }

    #[test]
    fn blank_metric_name_is_rejected() {
        assert!(
            validation_error(&mutate("/0/config/query/metric_name", json!(" ")))
                .contains("config.query.metric_name")
        );
    }

    #[test]
    fn too_many_dimensions_are_rejected() {
        let dimensions = (0..31)
            .map(|index| (format!("Dimension{index}"), json!("value")))
            .collect::<serde_json::Map<_, _>>();
        assert!(
            validation_error(&mutate("/0/config/query/dimensions", json!(dimensions)))
                .contains("config.query.dimensions")
        );
    }

    #[test]
    fn thirty_dimensions_are_accepted() {
        let dimensions = (0..30)
            .map(|index| (format!("Dimension{index}"), json!("value")))
            .collect::<serde_json::Map<_, _>>();
        mutate("/0/config/query/dimensions", json!(dimensions))
            .validate(&chat(&["critical", "warning", "info"]))
            .unwrap();
    }

    #[test]
    fn blank_dimension_name_is_rejected() {
        assert!(
            validation_error(&mutate("/0/config/query/dimensions", json!({" ": "value"})))
                .contains("config.query.dimensions")
        );
    }

    #[test]
    fn blank_dimension_value_is_rejected() {
        assert!(
            validation_error(&mutate("/0/config/query/dimensions/InstanceId", json!(" ")))
                .contains("config.query.dimensions")
        );
    }

    #[test]
    fn unsupported_period_is_rejected() {
        assert!(
            validation_error(&mutate("/0/config/query/period", json!(25)))
                .contains("config.query.period")
        );
    }

    #[test]
    fn period_above_one_day_is_rejected() {
        assert!(
            validation_error(&mutate("/0/config/query/period", json!(86460)))
                .contains("config.query.period")
        );
    }

    #[test]
    fn one_second_period_is_accepted() {
        mutate("/0/config/query/period", json!(1))
            .validate(&chat(&["critical", "warning", "info"]))
            .unwrap();
    }

    #[test]
    fn five_second_period_is_accepted() {
        mutate("/0/config/query/period", json!(5))
            .validate(&chat(&["critical", "warning", "info"]))
            .unwrap();
    }

    #[test]
    fn ten_second_period_is_accepted() {
        mutate("/0/config/query/period", json!(10))
            .validate(&chat(&["critical", "warning", "info"]))
            .unwrap();
    }

    #[test]
    fn twenty_second_period_is_accepted() {
        mutate("/0/config/query/period", json!(20))
            .validate(&chat(&["critical", "warning", "info"]))
            .unwrap();
    }

    #[test]
    fn thirty_second_period_is_accepted() {
        mutate("/0/config/query/period", json!(30))
            .validate(&chat(&["critical", "warning", "info"]))
            .unwrap();
    }

    #[test]
    fn one_day_period_is_accepted() {
        mutate("/0/config/query/period", json!(86400))
            .validate(&chat(&["critical", "warning", "info"]))
            .unwrap();
    }

    #[test]
    fn one_day_evaluation_window_is_accepted() {
        mutate(
            "/0/config/thresholds/severities/critical/evaluation_periods",
            json!(1440),
        )
        .validate(&chat(&["critical", "warning", "info"]))
        .unwrap();
    }

    #[test]
    fn end_offset_above_one_hour_is_rejected() {
        assert!(
            validation_error(&mutate("/0/config/query/end_offset_seconds", json!(3601)))
                .contains("config.query.end_offset_seconds")
        );
    }

    #[test]
    fn one_hour_end_offset_is_accepted() {
        mutate("/0/config/query/end_offset_seconds", json!(3600))
            .validate(&chat(&["critical", "warning", "info"]))
            .unwrap();
    }

    #[test]
    fn blank_id_is_rejected() {
        let error = validation_error(&mutate("/0/id", json!(" ")));
        assert!(error.contains("rule ` ` field `id`"));
    }

    #[test]
    fn duplicate_id_is_rejected() {
        let mut settings = settings();
        settings.rules.push(settings.rules[0].clone());
        let error = validation_error(&settings);
        assert!(error.contains("field `rules.id`"));
    }

    #[test]
    fn non_sixty_second_interval_is_rejected() {
        let error = validation_error(&mutate("/0/config/interval_seconds", json!(300)));
        assert!(error.contains("interval_seconds"));
    }

    #[test]
    fn empty_severities_are_rejected() {
        let error = validation_error(&mutate("/0/config/thresholds/severities", json!({})));
        assert!(error.contains("severities"));
    }

    #[test]
    fn zero_evaluation_periods_are_rejected() {
        let error = validation_error(&mutate(
            "/0/config/thresholds/severities/critical/evaluation_periods",
            json!(0),
        ));
        assert!(error.contains("evaluation_periods"));
    }

    #[test]
    fn zero_datapoints_to_alarm_is_rejected() {
        let error = validation_error(&mutate(
            "/0/config/thresholds/severities/critical/datapoints_to_alarm",
            json!(0),
        ));
        assert!(error.contains("datapoints_to_alarm"));
    }

    #[test]
    fn datapoints_above_evaluation_periods_are_rejected() {
        let error = validation_error(&mutate(
            "/0/config/thresholds/severities/critical/datapoints_to_alarm",
            json!(2),
        ));
        assert!(error.contains("datapoints_to_alarm"));
    }

    #[test]
    fn zero_period_is_rejected() {
        let error = validation_error(&mutate("/0/config/query/period", json!(0)));
        assert!(error.contains("config.query.period"));
    }

    #[test]
    fn oversized_period_product_is_rejected() {
        let error = validation_error(&mutate(
            "/0/config/thresholds/severities/critical/evaluation_periods",
            json!(1441),
        ));
        assert!(error.contains("evaluation_periods × config.query.period"));
    }

    #[test]
    fn missing_destination_fails_validation() {
        let error = settings()
            .validate(&chat(&["critical", "warning"]))
            .unwrap_err();
        assert!(error.to_string().contains("destinations.info"));
    }

    #[test]
    fn configured_destinations_are_accepted() {
        settings()
            .validate(&chat(&["critical", "warning", "info"]))
            .unwrap();
    }

    #[test]
    fn bad_grafana_scheme_is_rejected() {
        let mut settings = settings();
        settings.grafana.base_url = url::Url::parse("ftp://grafana.example").unwrap();
        let error = validation_error(&settings);
        assert!(error.contains("grafana.base_url"));
    }

    #[test]
    fn blank_cloudwatch_uid_is_rejected() {
        let mut settings = settings();
        settings.datasources.cloudwatch = " ".to_owned();
        let error = validation_error(&settings);
        assert!(error.contains("datasources.cloudwatch"));
    }

    #[test]
    fn blank_uid_is_rejected_even_without_rules() {
        let mut settings = settings();
        settings.rules.clear();
        settings.datasources.cloudwatch.clear();
        assert!(validation_error(&settings).contains("datasources.cloudwatch"));
    }

    #[test]
    fn configured_datasource_names_follow_settings() {
        let mut settings = settings();
        assert_eq!(settings.datasources.configured_names(), vec!["cloudwatch"]);
        settings.datasources.cloudwatch.clear();
        assert!(settings.datasources.configured_names().is_empty());
    }

    #[test]
    fn destinations_use_the_domain_level_conversion() {
        let destinations = settings().destinations;
        assert_eq!(destinations.for_severity(Level::Critical), "critical");
        assert_eq!(destinations.for_severity(Level::Warning), "warning");
        assert_eq!(destinations.for_severity(Level::Info), "info");
        assert_eq!(destinations.for_severity(Level::Normal), "");
    }

    #[test]
    fn configured_rule_deserializes_from_one_json_string() {
        assert_eq!(settings().rules().len(), 1);
    }
}

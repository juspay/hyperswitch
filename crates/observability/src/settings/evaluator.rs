use std::collections::HashSet;

use common_types::alert_rules::{AlertDatasource, AlertRuleConfig};
use common_utils::errors::CustomResult;
use error_stack::{Report, ResultExt};
use hyperswitch_masking::Secret;
use serde::{de::Error as _, Deserialize, Deserializer};

use crate::{
    domain::evaluator::{cloudwatch, Level, RuleDefinition},
    errors::ConfigurationError,
    settings::ChatSettings,
};

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvaluatorSettings {
    pub grafana: cloudwatch::GrafanaCloudwatchConfig,
    pub destinations: EvaluatorDestinations,
    #[serde(deserialize_with = "deserialize_rules")]
    rules: Vec<RuleDefinition>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvaluatorDestinations {
    pub critical: String,
    pub warning: String,
    pub info: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
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
        cloudwatch::validate_config(&self.grafana)?;
        for (field, destination) in [
            ("destinations.critical", &self.destinations.critical),
            ("destinations.warning", &self.destinations.warning),
            ("destinations.info", &self.destinations.info),
        ] {
            if !chat.destinations.contains_key(destination) {
                return Err(configuration_error(
                    field,
                    format!("names unconfigured chat destination `{destination}`"),
                ));
            }
        }
        Ok(())
    }

    pub(crate) fn rules(&self) -> &[RuleDefinition] {
        &self.rules
    }

    pub(crate) fn with_api_token(mut self, api_token: Secret<String>) -> Self {
        self.grafana.client.api_token = api_token;
        self
    }
}

impl EvaluatorDestinations {
    pub(crate) fn get(&self, level: Level) -> Option<&str> {
        match level {
            Level::Critical => Some(&self.critical),
            Level::Warning => Some(&self.warning),
            Level::Info => Some(&self.info),
            Level::Normal => None,
        }
    }
}

fn deserialize_rules<'de, D>(deserializer: D) -> Result<Vec<RuleDefinition>, D::Error>
where
    D: Deserializer<'de>,
{
    let raw = String::deserialize(deserializer)?;
    parse_rules(&raw).map_err(|error| D::Error::custom(format!("{error:?}")))
}

fn parse_rules(raw: &str) -> CustomResult<Vec<RuleDefinition>, ConfigurationError> {
    let configured: Vec<ConfiguredRule> = serde_json::from_str(raw).change_context(
        ConfigurationError::ConfigParsingError("invalid evaluator rules JSON".to_owned()),
    )?;
    let mut ids = HashSet::with_capacity(configured.len());
    let mut rules = Vec::with_capacity(configured.len());
    for configured_rule in configured {
        let rule = RuleDefinition::from(configured_rule);
        validate_rule(&rule, &mut ids)?;
        rules.push(rule);
    }
    Ok(rules)
}

fn validate_rule(
    rule: &RuleDefinition,
    ids: &mut HashSet<String>,
) -> CustomResult<(), ConfigurationError> {
    if rule.id.trim().is_empty() {
        return Err(rule_error(rule, "id", "must not be blank"));
    }
    if !ids.insert(rule.id.clone()) {
        return Err(rule_error(rule, "id", "must be unique"));
    }
    let config = rule.config.inner();
    if !(60..=86_400).contains(&config.interval_seconds) || config.interval_seconds % 60 != 0 {
        return Err(rule_error(
            rule,
            "config.interval_seconds",
            "must be a whole number of minutes from 60 through 86400",
        ));
    }
    if config.thresholds.severities.is_empty() {
        return Err(rule_error(
            rule,
            "config.thresholds.severities",
            "must contain at least one severity",
        ));
    }
    for (severity, threshold) in &config.thresholds.severities {
        if threshold.evaluation_periods == 0 {
            return Err(rule_error(
                rule,
                &format!("config.thresholds.severities.{severity}.evaluation_periods"),
                "must be at least 1",
            ));
        }
        if threshold
            .datapoints_to_alarm
            .is_some_and(|datapoints| datapoints == 0 || datapoints > threshold.evaluation_periods)
        {
            return Err(rule_error(
                rule,
                &format!("config.thresholds.severities.{severity}.datapoints_to_alarm"),
                "must be from 1 through evaluation_periods",
            ));
        }
    }
    match config.query.datasource() {
        AlertDatasource::Cloudwatch => cloudwatch::validate_rule(rule)?,
    }
    Ok(())
}

fn rule_error(rule: &RuleDefinition, field: &str, requirement: &str) -> Report<ConfigurationError> {
    Report::new(ConfigurationError::ConfigParsingError(
        "invalid evaluator rule".to_owned(),
    ))
    .attach_printable(format!("rule `{}` field `{field}` {requirement}", rule.id))
}

fn configuration_error(field: &str, requirement: String) -> ConfigurationError {
    ConfigurationError::ConfigParsingError(format!(
        "invalid evaluator configuration: field `{field}` {requirement}"
    ))
}

#[cfg(test)]
pub(crate) fn test_rule(
    id: &str,
    interval_seconds: u32,
) -> CustomResult<RuleDefinition, ConfigurationError> {
    let raw = serde_json::json!([{
        "id": id,
        "name": format!("Rule {id}"),
        "config": {
            "version": "v1",
            "classification": "test-alerts",
            "interval_seconds": interval_seconds,
            "query": {
                "datasource": "cloudwatch",
                "version": "v1",
                "region": "us-east-1",
                "namespace": "Test",
                "metric_name": "Value",
                "dimensions": {"InstanceId": "i-0123"},
                "statistic": "Average",
                "period": 60,
                "end_offset_seconds": 0
            },
            "thresholds": {"severities": {
                "critical": {
                    "threshold": 90,
                    "comparison_operator": "GreaterThanOrEqualToThreshold",
                    "evaluation_periods": 1,
                    "datapoints_to_alarm": 1,
                    "treat_missing_data": "missing",
                    "description": "Critical description"
                },
                "warning": {
                    "threshold": 80,
                    "comparison_operator": "GreaterThanOrEqualToThreshold",
                    "evaluation_periods": 1,
                    "datapoints_to_alarm": 1,
                    "treat_missing_data": "missing",
                    "description": "Warning description"
                },
                "info": {
                    "threshold": 70,
                    "comparison_operator": "GreaterThanOrEqualToThreshold",
                    "evaluation_periods": 1,
                    "datapoints_to_alarm": 1,
                    "treat_missing_data": "missing",
                    "description": "Info description"
                }
            }}
        }
    }])
    .to_string();
    parse_rules(&raw)?
        .into_iter()
        .next()
        .ok_or_else(|| rule_error_for_test(id))
}

#[cfg(test)]
fn rule_error_for_test(id: &str) -> Report<ConfigurationError> {
    Report::new(ConfigurationError::ConfigParsingError(
        "invalid evaluator test rule".to_owned(),
    ))
    .attach_printable(format!("rule `{id}` was not parsed"))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::settings::ChatDestination;

    type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

    fn valid_json() -> String {
        serde_json::json!([{
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
                    "dimensions": {"InstanceId": "i-0123"},
                    "statistic": "Average",
                    "period": 60
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

    fn parse_error(raw: &str) -> TestResult<String> {
        let Err(error) = parse_rules(raw) else {
            return Err("expected invalid rules".into());
        };
        Ok(format!("{error:?}"))
    }

    fn mutate(pointer: &str, replacement: serde_json::Value) -> TestResult<String> {
        let mut value: serde_json::Value = serde_json::from_str(&valid_json())?;
        let Some(field) = value.pointer_mut(pointer) else {
            return Err(format!("fixture is missing `{pointer}`").into());
        };
        *field = replacement;
        parse_error(&value.to_string())
    }

    fn settings() -> TestResult<EvaluatorSettings> {
        Ok(serde_json::from_value(serde_json::json!({
            "grafana": {
                "base_url": "https://grafana.example",
                "api_token": "token",
                "cloudwatch_datasource_uid": "cloudwatch"
            },
            "destinations": {
                "critical": "critical",
                "warning": "warning",
                "info": "info"
            },
            "rules": valid_json()
        }))?)
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

    #[test]
    fn valid_rule_preserves_dimension_case() -> TestResult {
        let rules = parse_rules(&valid_json())?;
        assert_eq!(rules.len(), 1);
        assert!(valid_json().contains("InstanceId"));
        Ok(())
    }

    #[test]
    fn blank_id_names_rule_and_field() -> TestResult {
        let error = mutate("/0/id", serde_json::json!(" "))?;
        assert!(error.contains("rule ` `") && error.contains("field `id`"));
        Ok(())
    }

    #[test]
    fn duplicate_id_names_rule_and_field() -> TestResult {
        let mut value: serde_json::Value = serde_json::from_str(&valid_json())?;
        let Some(duplicate) = value.as_array().and_then(|rules| rules.first()).cloned() else {
            return Err("rules fixture has no first rule".into());
        };
        let Some(rules) = value.as_array_mut() else {
            return Err("rules fixture is not an array".into());
        };
        rules.push(duplicate);
        let error = parse_error(&value.to_string())?;
        assert!(error.contains("rule `rule-1`") && error.contains("field `id`"));
        Ok(())
    }

    #[test]
    fn invalid_intervals_name_rule_and_field() -> TestResult {
        for interval in [0, 30, 86_401] {
            let error = mutate("/0/config/interval_seconds", serde_json::json!(interval))?;
            assert!(error.contains("rule `rule-1`") && error.contains("interval_seconds"));
        }
        Ok(())
    }

    #[test]
    fn empty_severities_name_rule_and_field() -> TestResult {
        let error = mutate("/0/config/thresholds/severities", serde_json::json!({}))?;
        assert!(error.contains("rule `rule-1`") && error.contains("severities"));
        Ok(())
    }

    #[test]
    fn zero_evaluation_periods_names_rule_and_field() -> TestResult {
        let error = mutate(
            "/0/config/thresholds/severities/critical/evaluation_periods",
            serde_json::json!(0),
        )?;
        assert!(error.contains("rule `rule-1`") && error.contains("evaluation_periods"));
        Ok(())
    }

    #[test]
    fn invalid_datapoints_to_alarm_names_rule_and_field() -> TestResult {
        for datapoints in [0, 2] {
            let error = mutate(
                "/0/config/thresholds/severities/critical/datapoints_to_alarm",
                serde_json::json!(datapoints),
            )?;
            assert!(error.contains("rule `rule-1`") && error.contains("datapoints_to_alarm"));
        }
        Ok(())
    }

    #[test]
    fn oversized_period_product_names_rule_and_field() -> TestResult {
        let mut value: serde_json::Value = serde_json::from_str(&valid_json())?;
        let Some(period) = value.pointer_mut("/0/config/query/period") else {
            return Err("fixture is missing query period".into());
        };
        *period = serde_json::json!(43_201);
        let Some(evaluation_periods) =
            value.pointer_mut("/0/config/thresholds/severities/critical/evaluation_periods")
        else {
            return Err("fixture is missing evaluation periods".into());
        };
        *evaluation_periods = serde_json::json!(2);
        let error = parse_error(&value.to_string())?;
        assert!(error.contains("rule `rule-1`") && error.contains("query.period"));
        Ok(())
    }

    #[test]
    fn missing_destination_fails_configuration_validation() -> TestResult {
        let error = settings()?.validate(&chat(&["critical", "warning"]));
        let Err(error) = error else {
            return Err("expected a missing destination error".into());
        };
        assert!(format!("{error:?}").contains("destinations.info"));
        Ok(())
    }

    #[test]
    fn configured_destinations_are_accepted() -> TestResult {
        settings()?.validate(&chat(&["critical", "warning", "info"]))?;
        Ok(())
    }

    #[test]
    fn configured_rule_deserializes_from_one_json_string() -> TestResult {
        let value = serde_json::json!({
            "grafana": {
                "base_url": "https://grafana.example",
                "api_token": "token",
                "cloudwatch_datasource_uid": "cloudwatch"
            },
            "destinations": {
                "critical": "critical",
                "warning": "warning",
                "info": "info"
            },
            "rules": valid_json()
        });
        let parsed: EvaluatorSettings = serde_json::from_value(value)?;
        assert_eq!(parsed.rules().len(), 1);
        Ok(())
    }
}

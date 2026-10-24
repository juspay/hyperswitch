use std::collections::{HashMap, HashSet};

use common_types::alert_rules::MetricSource;
use common_utils::fp_utils::when;
use external_services::metrics_provider::{
    grafana::{cloudwatch::GrafanaCloudwatch, GrafanaConfig},
    MetricsError, MetricsProvider,
};
use hyperswitch_masking::PeekInterface;
use serde::Deserialize;

use crate::{
    domain_models::evaluator::RuleDefinition, errors::ConfigurationError, settings::ChatSettings,
};

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AlertSettings {
    pub grafana: GrafanaConfig,
    pub datasources: HashMap<MetricSource, DatasourceSettings>,
    /// Where the evaluator reads rules from. Exactly one source is used.
    pub rule_source: RuleSource,
    #[serde(deserialize_with = "common_utils::custom_serde::json_string::deserialize")]
    pub(crate) rules: Vec<RuleDefinition>,
}

/// Where the evaluator reads rules from.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleSource {
    /// `rules` in this section, every rule on every tick.
    #[default]
    Config,
    /// The `alert_rules` table, managed through `/alerts/rules`; each rule when it is due.
    Database,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DatasourceSettings {
    pub uid: String,
}

impl AlertSettings {
    pub(crate) fn validate(&self, chat: &ChatSettings) -> Result<(), ConfigurationError> {
        // Rules here would be silently ignored, so refuse them rather than surprise someone.
        when(
            self.rule_source == RuleSource::Database && !self.rules.is_empty(),
            || {
                Err(ConfigurationError::ConfigParsingError(
                    "invalid alert configuration: field `rules` must be empty when `rule_source` is `database`".to_owned(),
                ))
            },
        )?;
        // Rules from the database arrive after startup, so Grafana must be usable from the start.
        if !self.rules.is_empty() || self.rule_source == RuleSource::Database {
            when(
                !url::Url::parse(&self.grafana.base_url)
                    .is_ok_and(|url| matches!(url.scheme(), "http" | "https")),
                || {
                    Err(ConfigurationError::ConfigParsingError(
                        "invalid alert configuration: field `grafana.base_url` must be a valid http or https URL".to_owned(),
                    ))
                },
            )?;
            when(self.grafana.api_token.peek().trim().is_empty(), || {
                Err(ConfigurationError::ConfigParsingError(
                    "invalid alert configuration: field `grafana.api_token` must not be blank"
                        .to_owned(),
                ))
            })?;
        }
        for (kind, datasource) in &self.datasources {
            when(datasource.uid.trim().is_empty(), || {
                Err(ConfigurationError::ConfigParsingError(format!(
                    "invalid alert configuration: field `datasources.{kind}.uid` must not be blank"
                )))
            })?;
        }
        let ids = self
            .rules
            .iter()
            .map(|rule| &rule.id)
            .collect::<HashSet<_>>();
        when(ids.len() != self.rules.len(), || {
            Err(ConfigurationError::ConfigParsingError(
                "invalid alert configuration: field `rules.id` must be unique".to_owned(),
            ))
        })?;
        for rule in &self.rules {
            when(rule.id.trim().is_empty(), || {
                Err(ConfigurationError::ConfigParsingError(format!(
                    "invalid alert configuration: rule `{}` field `id` must not be blank",
                    rule.id
                )))
            })?;
            when(rule.config.interval_seconds != 60, || {
                Err(ConfigurationError::ConfigParsingError(format!(
                    "invalid alert configuration: rule `{}` field `config.interval_seconds` must equal 60", rule.id
                )))
            })?;
            when(rule.config.thresholds.severities.is_empty(), || {
                Err(ConfigurationError::ConfigParsingError(format!(
                    "invalid alert configuration: rule `{}` field `config.thresholds.severities` must contain at least one severity", rule.id
                )))
            })?;
            for (severity, threshold) in &rule.config.thresholds.severities {
                when(!threshold.threshold.is_finite(), || {
                    Err(ConfigurationError::ConfigParsingError(format!(
                        "invalid alert configuration: rule `{}` field `config.thresholds.severities.{severity}.threshold` must be finite", rule.id
                    )))
                })?;
                when(
                    threshold.destination.trim().is_empty()
                        || !chat.destinations.contains_key(&threshold.destination),
                    || {
                        Err(ConfigurationError::ConfigParsingError(format!(
                            "invalid alert configuration: rule `{}` field `config.thresholds.severities.{severity}.destination` must name an existing notifier destination ID", rule.id
                        )))
                    },
                )?;
            }
            let datasource = self.datasources.get(&MetricSource::GrafanaCloudwatch)
                .ok_or_else(|| ConfigurationError::ConfigParsingError(format!(
                    "invalid alert configuration: rule `{}` field `config.query.datasource` requires its configured provider/datasource UID", rule.id
                )))?;
            if let Err(error) =
                GrafanaCloudwatch::validate_query(&rule.metrics_query(&datasource.uid))
            {
                let (field, requirement) = match error.current_context() {
                    MetricsError::InvalidQuery { field, requirement } => (*field, *requirement),
                    _ => ("query", "must be a valid metric query"),
                };
                return Err(ConfigurationError::ConfigParsingError(format!(
                    "invalid alert configuration: rule `{}` field `config.query.{field}` {requirement}", rule.id
                )));
            }
            let common_types::alert_rules::AlertQuery::GrafanaCloudwatch(query) =
                &rule.config.query;
            when(query.end_offset_seconds > 3600, || {
                Err(ConfigurationError::ConfigParsingError(format!(
                    "invalid alert configuration: rule `{}` field `config.query.end_offset_seconds` must not exceed 3600 seconds", rule.id
                )))
            })?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::settings::{ChatDestination, ChatSettings};

    #[test]
    fn alert_settings_default_to_no_rules_or_provider_configuration() {
        let settings: AlertSettings = serde_json::from_str("{}").unwrap();
        assert!(settings.rules.is_empty());
        assert!(settings.datasources.is_empty());
        assert!(settings.grafana.base_url.is_empty());
        assert_eq!(settings.rule_source, RuleSource::Config);
    }

    #[test]
    fn database_rules_need_grafana_and_no_config_rules() {
        let chat = ChatSettings::default();
        let settings =
            |value: serde_json::Value| -> AlertSettings { serde_json::from_value(value).unwrap() };

        let without_grafana = settings(serde_json::json!({ "rule_source": "database" }));
        let error = without_grafana.validate(&chat).unwrap_err().to_string();
        assert!(error.contains("grafana.base_url"), "{error}");

        let with_grafana = settings(serde_json::json!({
            "rule_source": "database",
            "grafana": { "base_url": "http://localhost:3000", "api_token": "token" },
            "datasources": { "grafana_cloudwatch": { "uid": "cloudwatch" } }
        }));
        with_grafana.validate(&chat).unwrap();

        let with_config_rules = settings(serde_json::json!({
            "rule_source": "database",
            "grafana": { "base_url": "http://localhost:3000", "api_token": "token" },
            "rules": "[{\"id\":\"cpu\",\"name\":\"CPU\",\"config\":{\"interval_seconds\":60,\"query\":{\"datasource\":\"grafana_cloudwatch\",\"region\":\"us-east-1\",\"namespace\":\"AWS/EC2\",\"metric_name\":\"CPUUtilization\",\"statistic\":\"Average\",\"period\":60},\"thresholds\":{\"severities\":{\"critical\":{\"threshold\":90,\"comparison_operator\":\"GreaterThanThreshold\",\"description\":\"CPU\",\"destination\":\"oncall\"}}}}}]"
        }));
        let error = with_config_rules.validate(&chat).unwrap_err().to_string();
        assert!(error.contains("field `rules` must be empty"), "{error}");
    }

    #[test]
    fn rules_decode_from_json_strings_and_preserve_parse_errors() {
        let settings: AlertSettings =
            serde_json::from_value(serde_json::json!({ "rules": "[]" })).unwrap();
        assert!(settings.rules.is_empty());

        let error = serde_json::from_value::<AlertSettings>(serde_json::json!({ "rules": "[" }))
            .unwrap_err()
            .to_string();
        assert!(error.contains("EOF while parsing a list"));
        assert!(error.contains("line 1 column 1"));
    }

    #[test]
    fn configured_rules_reuse_provider_query_validation() {
        let settings: AlertSettings = serde_json::from_value(serde_json::json!({
            "grafana": { "base_url": "http://localhost:3000", "api_token": "token" },
            "datasources": { "grafana_cloudwatch": { "uid": "cloudwatch" } },
            "rules": serde_json::to_string(&serde_json::json!([{
                "id": "requests",
                "name": "Requests",
                "config": {
                    "interval_seconds": 60,
                    "query": {
                        "datasource": "grafana_cloudwatch",
                        "region": " ",
                        "namespace": "AWS/ApplicationELB",
                        "metric_name": "RequestCount",
                        "statistic": "Sum",
                        "period": 60
                    },
                    "thresholds": { "severities": {
                        "critical": {
                            "threshold": 1.0,
                            "comparison_operator": "GreaterThanThreshold",
                            "description": "requests",
                            "destination": "oncall"
                        }
                    }}
                }
            }])).unwrap()
        }))
        .unwrap();
        let chat = ChatSettings {
            destinations: HashMap::from([("oncall".to_owned(), ChatDestination::Log)]),
            ..Default::default()
        };
        let error = settings.validate(&chat).unwrap_err().to_string();
        assert!(error.contains("config.query.region"));
        assert!(error.contains("must not be blank"));
    }
}

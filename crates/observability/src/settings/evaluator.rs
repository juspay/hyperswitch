use std::collections::{HashMap, HashSet};

use common_types::alert_rules::MetricSource;
use common_utils::fp_utils::when;
use external_services::metrics_provider::grafana::GrafanaConfig;
use hyperswitch_masking::PeekInterface;
use serde::Deserialize;

use crate::{
    domain_models::evaluator::RuleDefinition, errors::ConfigurationError, settings::ChatSettings,
};

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AlertSettings {
    pub rule_source: RuleSource,
    pub grafana: GrafanaConfig,
    pub datasources: HashMap<MetricSource, DatasourceSettings>,
    #[serde(deserialize_with = "common_utils::custom_serde::json_string::deserialize")]
    pub(crate) rules: Vec<RuleDefinition>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleSource {
    #[default]
    Config,
    Database,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DatasourceSettings {
    pub uid: String,
}

impl AlertSettings {
    pub(crate) fn validate(&self, chat: &ChatSettings) -> Result<(), ConfigurationError> {
        when(
            self.rule_source == RuleSource::Database && !self.rules.is_empty(),
            || {
                Err(ConfigurationError::ConfigParsingError(
                    "database rule_source requires empty config rules".to_owned(),
                ))
            },
        )?;
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
            when(
                !(60..=86400).contains(&rule.config.interval_seconds)
                    || rule.config.interval_seconds % 60 != 0,
                || {
                    Err(ConfigurationError::ConfigParsingError(format!(
                    "invalid alert configuration: rule `{}` field `config.interval_seconds` must be whole minutes from 60 to 86400 seconds", rule.id
                )))
                },
            )?;
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
            self.datasources.get(&rule.config.query.metric_source())
                .ok_or_else(|| ConfigurationError::ConfigParsingError(format!(
                    "invalid alert configuration: rule `{}` field `config.query.datasource` requires its configured provider/datasource UID", rule.id
                )))?;
            match &rule.config.query {
                common_types::alert_rules::AlertQuery::GrafanaCloudwatch(query) => {
                    when(query.end_offset_seconds > 3600, || {
                        Err(ConfigurationError::ConfigParsingError(format!(
                            "invalid alert configuration: rule `{}` field `config.query.end_offset_seconds` must not exceed 3600 seconds", rule.id
                        )))
                    })?;
                }
                common_types::alert_rules::AlertQuery::GrafanaLoki(query) => {
                    query.validate().map_err(|reason| {
                        ConfigurationError::ConfigParsingError(format!(
                            "invalid alert configuration: rule `{}` {reason}",
                            rule.id
                        ))
                    })?;
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::ChatDestination;

    #[test]
    fn alert_settings_default_to_no_rules_or_provider_configuration() {
        let settings: AlertSettings = serde_json::from_str("{}").unwrap();
        assert!(settings.rules.is_empty());
        assert!(settings.datasources.is_empty());
        assert!(settings.grafana.base_url.is_empty());
    }

    #[test]
    fn loki_only_settings_validate_and_reject_incomplete_query_contracts() {
        let settings: AlertSettings = serde_json::from_value(serde_json::json!({
            "grafana": {"base_url":"https://sandbox.example", "api_token":"test-token"},
            "datasources": {"grafana_loki":{"uid":"loki"}},
            "rules": serde_json::json!([{
                "id":"application-error", "name":"Application error", "config":{
                    "interval_seconds":60,
                    "query":{"datasource":"grafana_loki","expression":"count_over_time({app=\"test\"}[$__range])","mode":"instant","range_seconds":300,"reducer":"last"},
                    "thresholds":{"severities":{"critical":{"threshold":0,"comparison_operator":"GreaterThanThreshold","destination":"oncall","description":"Application error"}}}
                }
            }]).to_string()
        })).unwrap();
        let chat = ChatSettings {
            destinations: HashMap::from([("oncall".to_owned(), ChatDestination::Log)]),
            ..Default::default()
        };
        settings.validate(&chat).unwrap();
        for (mode, range, step, expression) in [
            (
                common_types::alert_rules::LokiQueryMode::Instant,
                0,
                None,
                "vector(1)",
            ),
            (
                common_types::alert_rules::LokiQueryMode::Range,
                300,
                None,
                "vector(1)",
            ),
            (
                common_types::alert_rules::LokiQueryMode::Instant,
                300,
                Some(0),
                "vector(1)",
            ),
            (
                common_types::alert_rules::LokiQueryMode::Instant,
                300,
                None,
                "$__interval",
            ),
            (
                common_types::alert_rules::LokiQueryMode::Instant,
                300,
                None,
                "$__unknown",
            ),
        ] {
            let mut invalid = settings.clone();
            let common_types::alert_rules::AlertQuery::GrafanaLoki(query) =
                &mut invalid.rules[0].config.query
            else {
                panic!("expected Loki");
            };
            query.mode = mode;
            query.range_seconds = range;
            query.step_seconds = step;
            query.expression = expression.to_owned();
            assert!(invalid.validate(&chat).is_err());
        }
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
    fn provider_query_fields_are_not_prevalidated_at_startup() {
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
        settings.validate(&chat).unwrap();
    }
}

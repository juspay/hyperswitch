use std::collections::{HashMap, HashSet};

use common_types::alert_rules::MetricSource;
use common_utils::fp_utils::when;
use error_stack::ResultExt;
use external_services::metrics_provider::{
    grafana::{cloudwatch::GrafanaCloudwatch, GrafanaConfig},
    MetricsError, MetricsProvider,
};
use hyperswitch_masking::PeekInterface;
use serde::{de::Error as _, Deserialize, Deserializer};

use crate::{
    domain_models::evaluator::RuleDefinition, errors::ConfigurationError, settings::ChatSettings,
};

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AlertSettings {
    pub grafana: GrafanaConfig,
    pub datasources: HashMap<MetricSource, DatasourceSettings>,
    #[serde(deserialize_with = "deserialize_rules")]
    pub(crate) rules: Vec<RuleDefinition>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DatasourceSettings {
    pub uid: String,
}

impl AlertSettings {
    pub(crate) fn validate(&self, chat: &ChatSettings) -> Result<(), ConfigurationError> {
        if !self.rules.is_empty() {
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

fn deserialize_rules<'de, D>(deserializer: D) -> Result<Vec<RuleDefinition>, D::Error>
where
    D: Deserializer<'de>,
{
    let raw = String::deserialize(deserializer)?;
    serde_json::from_str::<Vec<RuleDefinition>>(&raw)
        .change_context(ConfigurationError::ConfigParsingError(
            "invalid alert rules JSON".to_owned(),
        ))
        .map_err(|error| D::Error::custom(error.current_context()))
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

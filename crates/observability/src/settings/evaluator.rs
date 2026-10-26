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
    pub grafana: GrafanaConfig,
    pub datasources: HashMap<MetricSource, DatasourceSettings>,
    #[serde(deserialize_with = "common_utils::custom_serde::json_string::deserialize")]
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
            self.datasources.get(&MetricSource::GrafanaCloudwatch)
                .ok_or_else(|| ConfigurationError::ConfigParsingError(format!(
                    "invalid alert configuration: rule `{}` field `config.query.datasource` requires its configured provider/datasource UID", rule.id
                )))?;
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
    use super::*;

    #[test]
    fn alert_settings_default_to_no_rules_or_provider_configuration() {
        let settings: AlertSettings = serde_json::from_str("{}").unwrap();
        assert!(settings.rules.is_empty());
        assert!(settings.datasources.is_empty());
        assert!(settings.grafana.base_url.is_empty());
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
}

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
    /// More rules, in their own variable: one environment variable cannot exceed 128 KiB, and
    /// `rules` alone is close to that in prod.
    #[serde(
        default,
        deserialize_with = "common_utils::custom_serde::json_string::deserialize"
    )]
    pub(crate) loki_rules: Vec<RuleDefinition>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DatasourceSettings {
    pub uid: String,
}

impl AlertSettings {
    /// Every configured rule, from both variables.
    pub(crate) fn all_rules(&self) -> impl Iterator<Item = &RuleDefinition> {
        self.rules.iter().chain(&self.loki_rules)
    }

    pub(crate) fn validate(&self, chat: &ChatSettings) -> Result<(), ConfigurationError> {
        if self.all_rules().next().is_some() {
            when(
                !self
                    .grafana
                    .base_url
                    .as_ref()
                    .is_some_and(|url| matches!(url.scheme(), "http" | "https")),
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
            .all_rules()
            .map(|rule| &rule.id)
            .collect::<HashSet<_>>();
        when(ids.len() != self.all_rules().count(), || {
            Err(ConfigurationError::ConfigParsingError(
                "invalid alert configuration: field `rules.id` must be unique".to_owned(),
            ))
        })?;
        for rule in self.all_rules() {
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
            self.datasources.get(&rule.config.query.metric_source())
                .ok_or_else(|| ConfigurationError::ConfigParsingError(format!(
                    "invalid alert configuration: rule `{}` field `config.query.datasource` requires its configured provider/datasource UID", rule.id
                )))?;
            when(rule.config.query.end_offset_seconds() > 3600, || {
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
        assert!(settings.grafana.base_url.is_none());
        assert!(serde_json::from_value::<AlertSettings>(serde_json::json!({
            "grafana": {"base_url": "not a URL", "api_token": "test"}
        }))
        .is_err());
    }

    #[test]
    fn configured_rules_require_an_http_grafana_url() {
        let rule = serde_json::from_value(serde_json::json!({
            "id": "cpu", "name": "CPU",
            "config": {
                "interval_seconds": 60,
                "query": {
                    "datasource": "grafana_cloudwatch", "region": "us-east-1",
                    "namespace": "AWS/EC2", "metric_name": "CPUUtilization",
                    "statistic": "Average", "period": 60, "end_offset_seconds": 180
                },
                "thresholds": {"severities": {"critical": {
                    "threshold": 90,
                    "comparison_operator": "GreaterThanOrEqualToThreshold",
                    "destination": "smoke", "description": "CPU high"
                }}}
            }
        }))
        .unwrap();
        let mut settings = AlertSettings {
            rules: vec![rule],
            ..Default::default()
        };
        assert!(matches!(
            settings.validate(&ChatSettings::default()),
            Err(ConfigurationError::ConfigParsingError(detail)) if detail.contains("grafana.base_url")
        ));
        settings.grafana.base_url = Some(url::Url::parse("ftp://grafana.example.com").unwrap());
        assert!(matches!(
            settings.validate(&ChatSettings::default()),
            Err(ConfigurationError::ConfigParsingError(detail)) if detail.contains("grafana.base_url")
        ));
    }

    #[test]
    fn loki_rules_are_optional_and_combine_with_rules_under_unique_ids() {
        let rule = |id: &str, query: serde_json::Value| {
            serde_json::json!({
                "id": id,
                "name": id,
                "config": {
                    "interval_seconds": 60,
                    "query": query,
                    "thresholds": {"severities": {"critical": {
                        "threshold": 0,
                        "comparison_operator": "GreaterThanThreshold",
                        "destination": "smoke",
                        "description": "test"
                    }}}
                }
            })
        };
        let cloudwatch = rule(
            "cpu",
            serde_json::json!({
                "datasource": "grafana_cloudwatch", "region": "us-east-1",
                "namespace": "AWS/EC2", "metric_name": "CPUUtilization",
                "dimensions": {}, "statistic": "Average", "period": 60
            }),
        );
        let loki = |id: &str| {
            rule(
                id,
                serde_json::json!({
                    "datasource": "grafana_loki", "region": "us-east-1",
                    "expr": "sum(count_over_time({app=\"router\"}[1m]))", "period": 60
                }),
            )
        };
        let settings = |loki_rules: Option<serde_json::Value>| {
            let mut value = serde_json::json!({
                "grafana": {"base_url": "http://grafana.example.com", "api_token": "token"},
                "rules": serde_json::json!([cloudwatch]).to_string()
            });
            if let (Some(loki_rules), Some(object)) = (loki_rules, value.as_object_mut()) {
                object.insert(
                    "loki_rules".to_owned(),
                    serde_json::Value::String(loki_rules.to_string()),
                );
            }
            serde_json::from_value::<AlertSettings>(value).unwrap()
        };

        assert_eq!(settings(None).all_rules().count(), 1);

        let combined = settings(Some(serde_json::json!([loki("router-5xx")])));
        assert_eq!(
            combined
                .all_rules()
                .map(|rule| rule.id.as_str())
                .collect::<Vec<_>>(),
            ["cpu", "router-5xx"]
        );

        let duplicate = settings(Some(serde_json::json!([loki("cpu")])));
        assert!(matches!(
            duplicate.validate(&ChatSettings::default()),
            Err(ConfigurationError::ConfigParsingError(detail)) if detail.contains("rules.id")
        ));
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

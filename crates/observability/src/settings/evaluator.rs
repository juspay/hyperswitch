use std::collections::{HashMap, HashSet};

use common_types::alert_rules::{AlertDatasourceKind, AlertQuery};
use common_utils::fp_utils::when;
use error_stack::ResultExt;
use external_services::metrics_provider::grafana::GrafanaConfig;
use hyperswitch_masking::{PeekInterface, Secret};
use serde::{de::Error as _, Deserialize, Deserializer};

use crate::{
    domain::evaluator::{cloudwatch, RuleDefinition},
    errors::ConfigurationError,
    settings::ChatSettings,
};

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AlertSettings {
    pub grafana: GrafanaConfig,
    pub datasources: HashMap<AlertDatasourceKind, DatasourceSettings>,
    #[serde(deserialize_with = "deserialize_rules")]
    rules: Vec<RuleDefinition>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DatasourceSettings {
    pub uid: String,
}

impl AlertSettings {
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
        when(self.grafana.api_token.peek().trim().is_empty(), || {
            Err(configuration_error(
                None,
                "grafana.api_token",
                "must not be blank",
            ))
        })?;
        for (kind, datasource) in &self.datasources {
            when(datasource.uid.trim().is_empty(), || {
                Err(configuration_error(
                    None,
                    &format!("datasources.{kind}.uid"),
                    "must not be blank",
                ))
            })?;
        }
        let ids = self
            .rules
            .iter()
            .map(|rule| &rule.id)
            .collect::<HashSet<_>>();
        when(ids.len() != self.rules.len(), || {
            Err(configuration_error(None, "rules.id", "must be unique"))
        })?;
        for rule in &self.rules {
            when(rule.id.trim().is_empty(), || {
                Err(configuration_error(Some(rule), "id", "must not be blank"))
            })?;
            when(rule.config.interval_seconds != 60, || {
                Err(configuration_error(
                    Some(rule),
                    "config.interval_seconds",
                    "must equal 60",
                ))
            })?;
            when(
                self.datasource_uid(rule.config.query.datasource())
                    .is_none(),
                || {
                    Err(configuration_error(
                        Some(rule),
                        "config.query.datasource",
                        "requires its configured provider/datasource UID",
                    ))
                },
            )?;
            when(rule.config.thresholds.severities.is_empty(), || {
                Err(configuration_error(
                    Some(rule),
                    "config.thresholds.severities",
                    "must contain at least one severity",
                ))
            })?;
            for (severity, threshold) in &rule.config.thresholds.severities {
                when(!threshold.threshold.is_finite(), || {
                    Err(configuration_error(
                        Some(rule),
                        &format!("config.thresholds.severities.{severity}.threshold"),
                        "must be finite",
                    ))
                })?;
                when(
                    threshold.destination.trim().is_empty()
                        || !chat.destinations.contains_key(&threshold.destination),
                    || {
                        Err(configuration_error(
                            Some(rule),
                            &format!("config.thresholds.severities.{severity}.destination"),
                            "must name an existing notifier destination ID",
                        ))
                    },
                )?;
            }
            match &rule.config.query {
                AlertQuery::GrafanaCloudwatch(query) => {
                    cloudwatch::validate_query(query).map_err(|error| {
                        configuration_error(Some(rule), &error.field, error.requirement)
                    })?
                }
            }
        }
        Ok(())
    }

    pub(crate) fn rules(&self) -> &[RuleDefinition] {
        &self.rules
    }

    pub(crate) fn datasource_uid(&self, datasource: AlertDatasourceKind) -> Option<&str> {
        self.datasources
            .get(&datasource)
            .map(|settings| settings.uid.as_str())
    }

    pub(crate) fn configured_names(&self) -> Vec<String> {
        self.datasources.keys().map(ToString::to_string).collect()
    }

    pub(crate) fn with_api_token(mut self, api_token: Secret<String>) -> Self {
        self.grafana.api_token = api_token;
        self
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
        "invalid alert configuration: {field} {}",
        requirement.into()
    ))
}

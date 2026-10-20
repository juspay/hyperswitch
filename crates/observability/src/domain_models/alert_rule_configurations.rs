use api_models::observability::alert_manager::alert_rule_configurations as api;
use common_utils::generate_time_ordered_id;
use diesel_models::observability::alert_manager::alert_rule_configurations as storage;
use error_stack::report;
use time::PrimitiveDateTime;

use crate::errors::{ObservabilityApiResult, ObservabilityError};

const NAME_MAX_CHARS: usize = 255;

#[derive(Clone, Debug)]
pub struct AlertRuleConfigurationNew {
    pub id: String,
    pub name: String,
    pub is_enabled: bool,
    pub config: api::AlertRuleConfig,
}

#[derive(Clone, Debug)]
pub struct AlertRuleConfiguration {
    pub id: String,
    pub name: String,
    pub is_enabled: bool,
    pub config: api::AlertRuleConfig,
    pub next_run_at: PrimitiveDateTime,
    pub created_at: PrimitiveDateTime,
    pub last_updated_at: PrimitiveDateTime,
}

#[derive(Clone, Debug)]
pub enum AlertRuleConfigurationUpdate {
    Update {
        config: Option<api::AlertRuleConfig>,
        is_enabled: Option<bool>,
    },
}

fn invalid(message: String) -> error_stack::Report<ObservabilityError> {
    report!(ObservabilityError::InvalidRequest).attach_printable(message)
}

fn required_text(field: &str, value: &str) -> ObservabilityApiResult<()> {
    if value.trim().is_empty() {
        return Err(invalid(format!("{field} must not be blank")));
    }
    Ok(())
}

fn validate_config(config: &api::AlertRuleConfig) -> ObservabilityApiResult<()> {
    let config = config.inner();
    if config.frequency == 0 {
        return Err(invalid("frequency must be greater than zero".to_owned()));
    }
    if config.severities.is_empty() {
        return Err(invalid(
            "severities must contain at least one severity".to_owned(),
        ));
    }
    for (severity, rule) in &config.severities {
        if !rule.threshold.is_finite() {
            return Err(invalid(format!(
                "severities.{severity}.threshold must be finite"
            )));
        }
        required_text(
            &format!("severities.{severity}.description"),
            &rule.description,
        )?;
    }

    match config.query.inner() {
        api::AlertRuleQueryV1::Cloudwatch(query) => {
            required_text("query.region", &query.region)?;
            required_text("query.namespace", &query.namespace)?;
            required_text("query.metric_name", &query.metric_name)?;
            if query.period == 0 {
                return Err(invalid("query.period must be greater than zero".to_owned()));
            }
        }
    }

    Ok(())
}

impl TryFrom<api::AlertRuleConfigurationCreateRequest> for AlertRuleConfigurationNew {
    type Error = error_stack::Report<ObservabilityError>;

    fn try_from(request: api::AlertRuleConfigurationCreateRequest) -> Result<Self, Self::Error> {
        required_text("name", &request.name)?;
        if request.name.chars().count() > NAME_MAX_CHARS {
            return Err(invalid(format!(
                "name must be at most {NAME_MAX_CHARS} characters"
            )));
        }
        validate_config(&request.config)?;

        Ok(Self {
            id: generate_time_ordered_id("rule"),
            name: request.name,
            is_enabled: request.is_enabled.unwrap_or(true),
            config: request.config,
        })
    }
}

impl TryFrom<api::AlertRuleConfigurationUpdateRequest> for AlertRuleConfigurationUpdate {
    type Error = error_stack::Report<ObservabilityError>;

    fn try_from(request: api::AlertRuleConfigurationUpdateRequest) -> Result<Self, Self::Error> {
        if request.config.is_none() && request.is_enabled.is_none() {
            return Err(invalid("config or is_enabled must be provided".to_owned()));
        }
        if let Some(config) = request.config.as_ref() {
            validate_config(config)?;
        }

        Ok(Self::Update {
            config: request.config,
            is_enabled: request.is_enabled,
        })
    }
}

impl TryFrom<AlertRuleConfigurationNew> for storage::AlertRuleConfigurationNew {
    type Error = serde_json::Error;

    fn try_from(new: AlertRuleConfigurationNew) -> Result<Self, Self::Error> {
        Ok(Self {
            id: new.id,
            name: new.name,
            is_enabled: new.is_enabled,
            config: serde_json::to_value(new.config)?,
        })
    }
}

impl TryFrom<AlertRuleConfigurationUpdate> for storage::AlertRuleConfigurationUpdate {
    type Error = serde_json::Error;

    fn try_from(update: AlertRuleConfigurationUpdate) -> Result<Self, Self::Error> {
        match update {
            AlertRuleConfigurationUpdate::Update { config, is_enabled } => Ok(Self::Update {
                config: config.map(serde_json::to_value).transpose()?,
                is_enabled,
            }),
        }
    }
}

impl TryFrom<storage::AlertRuleConfiguration> for AlertRuleConfiguration {
    type Error = serde_json::Error;

    fn try_from(row: storage::AlertRuleConfiguration) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            name: row.name,
            is_enabled: row.is_enabled,
            config: serde_json::from_value(row.config)?,
            next_run_at: row.next_run_at,
            created_at: row.created_at,
            last_updated_at: row.last_updated_at,
        })
    }
}

impl From<AlertRuleConfiguration> for api::AlertRuleConfigurationResponse {
    fn from(rule: AlertRuleConfiguration) -> Self {
        Self {
            id: rule.id,
            name: rule.name,
            is_enabled: rule.is_enabled,
            config: rule.config,
            next_run_at: rule.next_run_at,
            created_at: rule.created_at,
            last_updated_at: rule.last_updated_at,
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::indexing_slicing, clippy::unwrap_used)]
mod tests {
    use std::collections::BTreeMap;

    use serde_json::{json, Value};
    use time::macros::datetime;

    use super::*;

    fn config_json() -> Value {
        json!({
            "version": "v1",
            "query": {
                "version": "v1",
                "data_source": "cloudwatch",
                "region": "us-east-1",
                "namespace": "AWS/EC2",
                "metric_name": "CPUUtilization",
                "dimensions": {"InstanceId": "i-053c2d51a2fc3535d"},
                "statistic": "Average",
                "period": 60
            },
            "frequency": 60,
            "severities": {
                "sev1": {
                    "operator": "gte",
                    "threshold": 80,
                    "description": "SEV1: Kafka broker 00 CPU utilization is above 80%."
                },
                "sev2": {
                    "operator": "gte",
                    "threshold": 70,
                    "description": "SEV2: Kafka broker 00 CPU utilization is above 70%."
                },
                "sev3": {
                    "operator": "gte",
                    "threshold": 60,
                    "description": "SEV3: Kafka broker 00 CPU utilization is above 60%."
                }
            }
        })
    }

    fn config() -> api::AlertRuleConfig {
        serde_json::from_value(config_json()).unwrap()
    }

    fn create_request() -> api::AlertRuleConfigurationCreateRequest {
        api::AlertRuleConfigurationCreateRequest {
            name: "Kafka broker CPU".to_owned(),
            config: config(),
            is_enabled: None,
        }
    }

    fn config_v1_mut(config: &mut api::AlertRuleConfig) -> &mut api::AlertRuleConfigV1 {
        match config {
            api::AlertRuleConfig::V1(config) => config,
        }
    }

    fn cloudwatch_mut(config: &mut api::AlertRuleConfig) -> &mut api::CloudwatchQueryV1 {
        match &mut config_v1_mut(config).query {
            api::AlertRuleQuery::V1(api::AlertRuleQueryV1::Cloudwatch(query)) => query,
        }
    }

    #[test]
    fn spec_json_round_trips_with_all_tags() {
        let expected = config_json();
        let parsed: api::AlertRuleConfig = serde_json::from_value(expected.clone()).unwrap();
        let serialized = serde_json::to_value(&parsed).unwrap();
        let reparsed: api::AlertRuleConfig = serde_json::from_value(serialized.clone()).unwrap();

        assert_eq!(parsed, reparsed);
        assert_eq!(serialized["version"], "v1");
        assert_eq!(serialized["query"]["version"], "v1");
        assert_eq!(serialized["query"]["data_source"], "cloudwatch");
        assert_eq!(serialized["severities"].as_object().unwrap().len(), 3);
        assert_eq!(parsed.inner().frequency, 60);
        assert_eq!(parsed.clone().into_inner().frequency, 60);
        assert!(matches!(
            parsed.inner().query.inner(),
            api::AlertRuleQueryV1::Cloudwatch(_)
        ));
        assert!(matches!(
            parsed.inner().query.clone().into_inner(),
            api::AlertRuleQueryV1::Cloudwatch(_)
        ));
    }

    #[test]
    fn enum_wire_values_match_the_contract() {
        for (statistic, expected) in [
            (api::CloudwatchStatistic::Average, "Average"),
            (api::CloudwatchStatistic::Maximum, "Maximum"),
            (api::CloudwatchStatistic::Minimum, "Minimum"),
            (api::CloudwatchStatistic::Sum, "Sum"),
        ] {
            assert_eq!(serde_json::to_value(statistic).unwrap(), expected);
        }
        for (operator, expected) in [
            (api::ComparisonOperator::Gt, "gt"),
            (api::ComparisonOperator::Gte, "gte"),
            (api::ComparisonOperator::Lt, "lt"),
            (api::ComparisonOperator::Lte, "lte"),
        ] {
            assert_eq!(serde_json::to_value(operator).unwrap(), expected);
            assert_eq!(operator.to_string(), expected);
        }
        for (severity, expected) in [
            (api::Severity::Sev1, "sev1"),
            (api::Severity::Sev2, "sev2"),
            (api::Severity::Sev3, "sev3"),
        ] {
            assert_eq!(serde_json::to_value(severity).unwrap(), expected);
            assert_eq!(severity.to_string(), expected);
        }
    }

    #[test]
    fn unsupported_tags_fail_to_parse() {
        let mut unknown_version = config_json();
        unknown_version["version"] = json!("v2");

        let mut unknown_query_version = config_json();
        unknown_query_version["query"]["version"] = json!("v2");

        let mut unknown_data_source = config_json();
        unknown_data_source["query"]["data_source"] = json!("prometheus");

        let mut unknown_severity = config_json();
        unknown_severity["severities"]["sev4"] = unknown_severity["severities"]["sev1"].clone();

        let mut unknown_operator = config_json();
        unknown_operator["severities"]["sev1"]["operator"] = json!("eq");

        for value in [
            unknown_version,
            unknown_query_version,
            unknown_data_source,
            unknown_severity,
            unknown_operator,
        ] {
            assert!(serde_json::from_value::<api::AlertRuleConfig>(value).is_err());
        }
    }

    #[test]
    fn unknown_fields_are_rejected_at_every_strict_level() {
        let mut config_field = config_json();
        config_field["extra"] = json!(true);

        let mut query_field = config_json();
        query_field["query"]["extra"] = json!(true);

        let mut severity_field = config_json();
        severity_field["severities"]["sev1"]["extra"] = json!(true);

        for value in [config_field, query_field, severity_field] {
            assert!(serde_json::from_value::<api::AlertRuleConfig>(value).is_err());
        }

        let request = json!({
            "name": "name",
            "config": config_json(),
            "extra": true
        });
        assert!(
            serde_json::from_value::<api::AlertRuleConfigurationCreateRequest>(request).is_err()
        );

        let update = json!({"is_enabled": true, "extra": true});
        assert!(
            serde_json::from_value::<api::AlertRuleConfigurationUpdateRequest>(update).is_err()
        );
    }

    #[test]
    fn create_defaults_enabled_and_mints_rule_id() {
        let new = AlertRuleConfigurationNew::try_from(create_request()).unwrap();

        assert!(new.id.starts_with("rule_"));
        assert_eq!(new.name, "Kafka broker CPU");
        assert!(new.is_enabled);
        assert_eq!(new.config, config());

        let mut request = create_request();
        request.is_enabled = Some(false);
        assert!(
            !AlertRuleConfigurationNew::try_from(request)
                .unwrap()
                .is_enabled
        );
    }

    #[test]
    fn blank_and_overlong_names_are_rejected_by_unicode_character_count() {
        let mut blank = create_request();
        blank.name = " \t\n ".to_owned();
        assert!(AlertRuleConfigurationNew::try_from(blank).is_err());

        let mut boundary = create_request();
        boundary.name = "界".repeat(NAME_MAX_CHARS);
        assert!(AlertRuleConfigurationNew::try_from(boundary).is_ok());

        let mut overlong = create_request();
        overlong.name = "界".repeat(NAME_MAX_CHARS + 1);
        assert!(AlertRuleConfigurationNew::try_from(overlong).is_err());
    }

    #[test]
    fn zero_frequency_is_rejected() {
        let mut request = create_request();
        config_v1_mut(&mut request.config).frequency = 0;
        assert!(AlertRuleConfigurationNew::try_from(request).is_err());
    }

    #[test]
    fn zero_period_is_rejected() {
        let mut request = create_request();
        cloudwatch_mut(&mut request.config).period = 0;
        assert!(AlertRuleConfigurationNew::try_from(request).is_err());
    }

    #[test]
    fn blank_cloudwatch_identifiers_are_rejected() {
        let mut blank_region = create_request();
        cloudwatch_mut(&mut blank_region.config).region = "  ".to_owned();

        let mut blank_namespace = create_request();
        cloudwatch_mut(&mut blank_namespace.config).namespace = "\n".to_owned();

        let mut blank_metric_name = create_request();
        cloudwatch_mut(&mut blank_metric_name.config).metric_name = "\t".to_owned();

        for request in [blank_region, blank_namespace, blank_metric_name] {
            assert!(AlertRuleConfigurationNew::try_from(request).is_err());
        }
    }

    #[test]
    fn empty_severities_are_rejected() {
        let mut request = create_request();
        config_v1_mut(&mut request.config).severities = BTreeMap::new();
        assert!(AlertRuleConfigurationNew::try_from(request).is_err());
    }

    #[test]
    fn every_non_finite_threshold_is_rejected() {
        for threshold in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            for severity in [
                api::Severity::Sev1,
                api::Severity::Sev2,
                api::Severity::Sev3,
            ] {
                let mut request = create_request();
                config_v1_mut(&mut request.config)
                    .severities
                    .get_mut(&severity)
                    .unwrap()
                    .threshold = threshold;
                assert!(AlertRuleConfigurationNew::try_from(request).is_err());
            }
        }
    }

    #[test]
    fn every_blank_severity_description_is_rejected() {
        for severity in [
            api::Severity::Sev1,
            api::Severity::Sev2,
            api::Severity::Sev3,
        ] {
            let mut request = create_request();
            config_v1_mut(&mut request.config)
                .severities
                .get_mut(&severity)
                .unwrap()
                .description = "  ".to_owned();
            assert!(AlertRuleConfigurationNew::try_from(request).is_err());
        }
    }

    #[test]
    fn update_requires_a_field_and_validates_config() {
        assert!(
            AlertRuleConfigurationUpdate::try_from(api::AlertRuleConfigurationUpdateRequest {
                config: None,
                is_enabled: None,
            })
            .is_err()
        );

        assert!(
            AlertRuleConfigurationUpdate::try_from(api::AlertRuleConfigurationUpdateRequest {
                config: None,
                is_enabled: Some(false),
            })
            .is_ok()
        );

        let mut invalid = config();
        cloudwatch_mut(&mut invalid).period = 0;
        assert!(
            AlertRuleConfigurationUpdate::try_from(api::AlertRuleConfigurationUpdateRequest {
                config: Some(invalid),
                is_enabled: None,
            })
            .is_err()
        );
    }

    #[test]
    fn storage_and_response_conversions_preserve_fields() {
        let new = AlertRuleConfigurationNew::try_from(create_request()).unwrap();
        let stored_new = storage::AlertRuleConfigurationNew::try_from(new).unwrap();
        assert_eq!(stored_new.name, "Kafka broker CPU");
        assert!(stored_new.is_enabled);
        assert_eq!(
            serde_json::from_value::<api::AlertRuleConfig>(stored_new.config.clone()).unwrap(),
            config()
        );

        let timestamp = datetime!(2026-09-15 09:45:00);
        let stored = storage::AlertRuleConfiguration {
            id: stored_new.id,
            name: stored_new.name,
            is_enabled: stored_new.is_enabled,
            config: stored_new.config,
            next_run_at: timestamp,
            created_at: timestamp,
            last_updated_at: timestamp,
        };
        let domain = AlertRuleConfiguration::try_from(stored).unwrap();
        let response = api::AlertRuleConfigurationResponse::from(domain);
        assert_eq!(response.config, config());
        assert_eq!(response.next_run_at, timestamp);
        assert_eq!(response.created_at, timestamp);
        assert_eq!(response.last_updated_at, timestamp);
        let serialized_response = serde_json::to_value(&response).unwrap();
        assert_eq!(
            serialized_response["next_run_at"],
            "2026-09-15T09:45:00.000Z"
        );
        assert_eq!(
            serialized_response["created_at"],
            "2026-09-15T09:45:00.000Z"
        );
        assert_eq!(
            serialized_response["last_updated_at"],
            "2026-09-15T09:45:00.000Z"
        );

        let update =
            AlertRuleConfigurationUpdate::try_from(api::AlertRuleConfigurationUpdateRequest {
                config: Some(config()),
                is_enabled: Some(false),
            })
            .unwrap();
        let storage_update = storage::AlertRuleConfigurationUpdate::try_from(update).unwrap();
        assert!(matches!(
            storage_update,
            storage::AlertRuleConfigurationUpdate::Update {
                config: Some(_),
                is_enabled: Some(false)
            }
        ));
    }

    #[test]
    fn malformed_stored_config_is_rejected() {
        let timestamp = datetime!(2026-09-15 09:45:00);
        let stored = storage::AlertRuleConfiguration {
            id: "rule_bad".to_owned(),
            name: "bad".to_owned(),
            is_enabled: true,
            config: json!({"version": "v2"}),
            next_run_at: timestamp,
            created_at: timestamp,
            last_updated_at: timestamp,
        };

        assert!(AlertRuleConfiguration::try_from(stored).is_err());
    }
}

use api_models::observability::alert_manager::alert_rules as api;
use common_enums::AlertRuleStatus;
use common_types::alert_rules::{
    AlertQuery, AlertQueryV1, AlertRuleConfig, AlertRuleConfigStoredV1, AlertRuleConfigV1,
    AlertThresholdsV1, CloudwatchQuery, CloudwatchQueryV1,
};
use diesel_models::observability::alert_manager::alert_rules as storage;
use error_stack::report;
use time::PrimitiveDateTime;

use crate::errors::{ObservabilityApiResult, ObservabilityError};

const ID_MAX_CHARS: usize = 64;
const NAME_MAX_CHARS: usize = 255;
const CLASSIFICATION_MAX_CHARS: usize = 64;
const DIMENSIONS_MAX: usize = 30;
const SECONDS_MAX: u32 = 86_400;

#[derive(Clone, Debug)]
pub struct AlertRuleNew {
    pub id: String,
    pub name: String,
    pub config: AlertRuleConfig,
    pub status: AlertRuleStatus,
}

#[derive(Clone, Debug)]
pub struct AlertRule {
    pub id: String,
    pub name: String,
    pub config: AlertRuleConfig,
    pub status: AlertRuleStatus,
    pub next_run_at: PrimitiveDateTime,
    pub last_evaluated_at: Option<PrimitiveDateTime>,
    pub created_at: PrimitiveDateTime,
    pub last_updated_at: PrimitiveDateTime,
}

#[derive(Clone, Debug)]
pub enum AlertRuleUpdate {
    Update {
        name: Option<String>,
        config: Option<AlertRuleConfig>,
        status: Option<AlertRuleStatus>,
    },
    NextRun {
        next_run_at: PrimitiveDateTime,
    },
    Evaluated {
        last_evaluated_at: PrimitiveDateTime,
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

fn bounded_text(field: &str, value: &str, max_chars: usize) -> ObservabilityApiResult<()> {
    required_text(field, value)?;
    if value.chars().count() > max_chars {
        return Err(invalid(format!(
            "{field} must be at most {max_chars} characters"
        )));
    }
    Ok(())
}

fn validate_id(id: &str) -> ObservabilityApiResult<()> {
    let mut chars = id.chars();
    let first = chars
        .next()
        .ok_or_else(|| invalid("id must not be blank".to_owned()))?;
    if id.chars().count() > ID_MAX_CHARS {
        return Err(invalid(format!(
            "id must be at most {ID_MAX_CHARS} characters"
        )));
    }
    if !first.is_ascii_lowercase() && !first.is_ascii_digit() {
        return Err(invalid(
            "id must start with a lowercase letter or digit".to_owned(),
        ));
    }
    if !chars.all(|character| {
        character.is_ascii_lowercase()
            || character.is_ascii_digit()
            || character == '-'
            || character == '_'
    }) {
        return Err(invalid(
            "id must contain only lowercase letters, digits, '-' or '_'".to_owned(),
        ));
    }
    Ok(())
}

fn validate_interval_seconds(interval_seconds: u32) -> ObservabilityApiResult<()> {
    if !(60..=SECONDS_MAX).contains(&interval_seconds) || interval_seconds % 60 != 0 {
        return Err(invalid(
            "config.interval_seconds must be a whole number of minutes from 60 to 86400".to_owned(),
        ));
    }
    Ok(())
}

fn validate_query(query: &CloudwatchQueryV1) -> ObservabilityApiResult<()> {
    required_text("config.query.region", &query.region)?;
    required_text("config.query.namespace", &query.namespace)?;
    required_text("config.query.metric_name", &query.metric_name)?;
    if query.dimensions.len() > DIMENSIONS_MAX {
        return Err(invalid(format!(
            "config.query.dimensions must contain at most {DIMENSIONS_MAX} entries"
        )));
    }
    for (name, value) in &query.dimensions {
        required_text("config.query.dimensions name", name)?;
        required_text(&format!("config.query.dimensions.{name}"), value)?;
    }
    if !matches!(query.period, 1 | 5 | 10 | 20 | 30)
        && (query.period == 0 || query.period > SECONDS_MAX || query.period % 60 != 0)
    {
        return Err(invalid(
            "config.query.period must be 1, 5, 10, 20, 30, or a multiple of 60 up to 86400"
                .to_owned(),
        ));
    }
    if query.end_offset_seconds > 3600 {
        return Err(invalid(
            "config.query.end_offset_seconds must be from 0 to 3600".to_owned(),
        ));
    }
    Ok(())
}

fn validate_thresholds(
    thresholds: &AlertThresholdsV1,
    query_period: u32,
) -> ObservabilityApiResult<()> {
    if thresholds.severities.is_empty() {
        return Err(invalid(
            "config.thresholds.severities must contain at least one severity".to_owned(),
        ));
    }
    for (severity, threshold) in &thresholds.severities {
        let field = format!("config.thresholds.severities.{severity}");
        if !threshold.threshold.is_finite() {
            return Err(invalid(format!("{field}.threshold must be finite")));
        }
        if threshold.evaluation_periods == 0 {
            return Err(invalid(format!(
                "{field}.evaluation_periods must be at least 1"
            )));
        }
        if !matches!(
            threshold.evaluation_periods.checked_mul(query_period),
            Some(seconds) if seconds <= SECONDS_MAX
        ) {
            return Err(invalid(format!(
                "{field}.evaluation_periods multiplied by config.query.period must be at most 86400"
            )));
        }
        if threshold
            .datapoints_to_alarm
            .is_some_and(|datapoints| datapoints == 0 || datapoints > threshold.evaluation_periods)
        {
            return Err(invalid(format!(
                "{field}.datapoints_to_alarm must be from 1 to evaluation_periods"
            )));
        }
        required_text(&format!("{field}.description"), &threshold.description)?;
    }
    Ok(())
}

fn validate_config(config: &AlertRuleConfigV1) -> ObservabilityApiResult<()> {
    bounded_text(
        "config.classification",
        &config.classification,
        CLASSIFICATION_MAX_CHARS,
    )?;
    validate_interval_seconds(config.interval_seconds)?;
    match &config.query {
        AlertQueryV1::Cloudwatch(query) => {
            validate_query(query)?;
            validate_thresholds(&config.thresholds, query.period)
        }
    }
}

fn into_stored_config(config: AlertRuleConfigV1) -> AlertRuleConfig {
    let AlertRuleConfigV1 {
        classification,
        interval_seconds,
        query,
        thresholds,
    } = config;
    let query = match query {
        AlertQueryV1::Cloudwatch(query) => AlertQuery::Cloudwatch(CloudwatchQuery::V1(query)),
    };
    AlertRuleConfig::V1(AlertRuleConfigStoredV1 {
        classification,
        interval_seconds,
        query,
        thresholds,
    })
}

fn into_api_config(config: AlertRuleConfig) -> AlertRuleConfigV1 {
    let AlertRuleConfigStoredV1 {
        classification,
        interval_seconds,
        query,
        thresholds,
    } = config.into_inner();
    let query = match query {
        AlertQuery::Cloudwatch(query) => AlertQueryV1::Cloudwatch(query.into_inner()),
    };
    AlertRuleConfigV1 {
        classification,
        interval_seconds,
        query,
        thresholds,
    }
}

impl TryFrom<api::AlertRuleCreateRequest> for AlertRuleNew {
    type Error = error_stack::Report<ObservabilityError>;

    fn try_from(request: api::AlertRuleCreateRequest) -> Result<Self, Self::Error> {
        validate_id(&request.id)?;
        bounded_text("name", &request.name, NAME_MAX_CHARS)?;
        validate_config(&request.config)?;

        Ok(Self {
            id: request.id,
            name: request.name,
            config: into_stored_config(request.config),
            status: request.status.unwrap_or(AlertRuleStatus::Active),
        })
    }
}

impl TryFrom<api::AlertRuleUpdateRequest> for AlertRuleUpdate {
    type Error = error_stack::Report<ObservabilityError>;

    fn try_from(request: api::AlertRuleUpdateRequest) -> Result<Self, Self::Error> {
        if request.name.is_none() && request.config.is_none() && request.status.is_none() {
            return Err(invalid("at least one field must be provided".to_owned()));
        }
        if let Some(name) = request.name.as_deref() {
            bounded_text("name", name, NAME_MAX_CHARS)?;
        }
        if let Some(config) = request.config.as_ref() {
            validate_config(config)?;
        }

        Ok(Self::Update {
            name: request.name,
            config: request.config.map(into_stored_config),
            status: request.status,
        })
    }
}

impl From<AlertRuleNew> for storage::AlertRuleNew {
    fn from(new: AlertRuleNew) -> Self {
        Self {
            id: new.id,
            name: new.name,
            config: new.config,
            status: new.status,
        }
    }
}

impl From<AlertRuleUpdate> for storage::AlertRuleUpdate {
    fn from(update: AlertRuleUpdate) -> Self {
        match update {
            AlertRuleUpdate::Update {
                name,
                config,
                status,
            } => Self::Update {
                name,
                config,
                status,
            },
            AlertRuleUpdate::NextRun { next_run_at } => Self::NextRun { next_run_at },
            AlertRuleUpdate::Evaluated { last_evaluated_at } => {
                Self::Evaluated { last_evaluated_at }
            }
        }
    }
}

impl From<storage::AlertRule> for AlertRule {
    fn from(row: storage::AlertRule) -> Self {
        Self {
            id: row.id,
            name: row.name,
            config: row.config,
            status: row.status,
            next_run_at: row.next_run_at,
            last_evaluated_at: row.last_evaluated_at,
            created_at: row.created_at,
            last_updated_at: row.last_updated_at,
        }
    }
}

impl From<AlertRule> for api::AlertRuleResponse {
    fn from(rule: AlertRule) -> Self {
        Self {
            id: rule.id,
            name: rule.name,
            config: into_api_config(rule.config),
            status: rule.status,
            next_run_at: rule.next_run_at,
            last_evaluated_at: rule.last_evaluated_at,
            created_at: rule.created_at,
            last_updated_at: rule.last_updated_at,
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::indexing_slicing, clippy::unwrap_used)]
mod tests {
    use std::collections::BTreeMap;

    use common_types::alert_rules::{Severity, SeverityThresholdV1, TreatMissingData};
    use serde_json::{json, Value};
    use time::macros::datetime;

    use super::*;

    fn api_config_json() -> Value {
        json!({
            "classification": "kafka-alerts",
            "interval_seconds": 60,
            "query": {
                "datasource": "cloudwatch",
                "region": "us-east-1",
                "namespace": "AWS/EC2",
                "metric_name": "CPUUtilization",
                "dimensions": {"InstanceId": "i-053c2d51a2fc3535d"},
                "statistic": "Average",
                "period": 60,
                "end_offset_seconds": 180
            },
            "thresholds": {
                "severities": {
                    "critical": {
                        "threshold": 90.0,
                        "comparison_operator": "GreaterThanOrEqualToThreshold",
                        "evaluation_periods": 1,
                        "treat_missing_data": "breaching",
                        "description": "CRITICAL: CPU utilization is above 90%."
                    },
                    "warning": {
                        "threshold": 80.0,
                        "comparison_operator": "GreaterThanOrEqualToThreshold",
                        "evaluation_periods": 3,
                        "datapoints_to_alarm": 2,
                        "treat_missing_data": "missing",
                        "description": "WARNING: CPU utilization is above 80%."
                    }
                }
            }
        })
    }

    fn config_value() -> AlertRuleConfigV1 {
        serde_json::from_value(api_config_json()).unwrap()
    }

    fn create_request() -> api::AlertRuleCreateRequest {
        api::AlertRuleCreateRequest {
            id: "kafka-broker-00-cpu".to_owned(),
            name: "Kafka broker CPU".to_owned(),
            config: config_value(),
            status: None,
        }
    }

    fn cloudwatch_mut(request: &mut api::AlertRuleCreateRequest) -> &mut CloudwatchQueryV1 {
        match &mut request.config.query {
            AlertQueryV1::Cloudwatch(query) => query,
        }
    }

    fn severity_mut(
        request: &mut api::AlertRuleCreateRequest,
        severity: Severity,
    ) -> &mut SeverityThresholdV1 {
        request
            .config
            .thresholds
            .severities
            .get_mut(&severity)
            .unwrap()
    }

    #[test]
    fn stored_json_round_trips_and_api_shape_hides_versions() {
        let mut stored_json = api_config_json();
        stored_json["version"] = json!("v1");
        stored_json["query"]["version"] = json!("v1");
        let stored: AlertRuleConfig = serde_json::from_value(stored_json.clone()).unwrap();
        assert!(matches!(
            &stored.inner().query,
            AlertQuery::Cloudwatch(CloudwatchQuery::V1(_))
        ));
        assert_eq!(serde_json::to_value(&stored).unwrap(), stored_json);

        let timestamp = datetime!(2026-09-15 09:45:00);
        let response = api::AlertRuleResponse::from(AlertRule {
            id: "rule".to_owned(),
            name: "Rule".to_owned(),
            config: stored,
            status: AlertRuleStatus::Active,
            next_run_at: timestamp,
            last_evaluated_at: None,
            created_at: timestamp,
            last_updated_at: timestamp,
        });
        let response_json = serde_json::to_value(response).unwrap();
        assert!(response_json["config"].get("version").is_none());
        assert!(response_json["config"]["query"].get("version").is_none());
        assert_eq!(response_json["config"]["query"]["datasource"], "cloudwatch");
    }

    #[test]
    fn nested_defaults_and_unknown_fields_match_the_contract() {
        let mut defaulted = api_config_json();
        defaulted["query"]
            .as_object_mut()
            .unwrap()
            .remove("end_offset_seconds");
        defaulted["thresholds"]["severities"]["warning"]
            .as_object_mut()
            .unwrap()
            .remove("treat_missing_data");
        defaulted["thresholds"]["severities"]["warning"]
            .as_object_mut()
            .unwrap()
            .remove("datapoints_to_alarm");
        let config: AlertRuleConfigV1 = serde_json::from_value(defaulted).unwrap();
        let AlertQueryV1::Cloudwatch(query) = config.query;
        assert_eq!(query.end_offset_seconds, 180);
        assert_eq!(
            config.thresholds.severities[&Severity::Warning].treat_missing_data,
            TreatMissingData::Missing
        );
        assert_eq!(
            config.thresholds.severities[&Severity::Warning].datapoints_to_alarm,
            None
        );

        for path in ["config", "query", "thresholds", "severity"] {
            let mut value = api_config_json();
            match path {
                "config" => value["extra"] = json!(true),
                "query" => value["query"]["extra"] = json!(true),
                "thresholds" => value["thresholds"]["extra"] = json!(true),
                "severity" => value["thresholds"]["severities"]["critical"]["extra"] = json!(true),
                _ => unreachable!(),
            }
            assert!(serde_json::from_value::<AlertRuleConfigV1>(value).is_err());
        }
    }

    #[test]
    fn unsupported_wire_values_fail_to_parse() {
        let mut severity = api_config_json();
        severity["thresholds"]["severities"]["sev1"] =
            severity["thresholds"]["severities"]["critical"].clone();
        severity["thresholds"]["severities"]
            .as_object_mut()
            .unwrap()
            .remove("critical");
        assert!(serde_json::from_value::<AlertRuleConfigV1>(severity).is_err());

        let mut missing_data = api_config_json();
        missing_data["thresholds"]["severities"]["critical"]["treat_missing_data"] =
            json!("ignore");
        assert!(serde_json::from_value::<AlertRuleConfigV1>(missing_data).is_err());

        let mut datasource = api_config_json();
        datasource["query"]["datasource"] = json!("prometheus");
        assert!(serde_json::from_value::<AlertRuleConfigV1>(datasource).is_err());

        for path in ["config", "query"] {
            let mut versioned_api = api_config_json();
            match path {
                "config" => versioned_api["version"] = json!("v1"),
                "query" => versioned_api["query"]["version"] = json!("v1"),
                _ => unreachable!(),
            }
            assert!(
                serde_json::from_value::<api::AlertRuleCreateRequest>(json!({
                    "id": "rule",
                    "name": "Rule",
                    "config": versioned_api
                }))
                .is_err()
            );
        }

        assert!(serde_json::from_value::<AlertRuleConfig>(json!({
            "version": "v2"
        }))
        .is_err());

        let mut unknown_query_version = api_config_json();
        unknown_query_version["version"] = json!("v1");
        unknown_query_version["query"]["version"] = json!("v9");
        assert!(serde_json::from_value::<AlertRuleConfig>(unknown_query_version).is_err());

        let mut missing_query_version = api_config_json();
        missing_query_version["version"] = json!("v1");
        assert!(serde_json::from_value::<AlertRuleConfig>(missing_query_version).is_err());
    }

    #[test]
    fn id_name_classification_and_interval_are_validated() {
        for id in ["a".to_owned(), "1".repeat(ID_MAX_CHARS)] {
            let mut request = create_request();
            request.id = id;
            assert!(AlertRuleNew::try_from(request).is_ok());
        }
        for id in [
            String::new(),
            "a".repeat(ID_MAX_CHARS + 1),
            "-rule".to_owned(),
            "_rule".to_owned(),
            "Rule".to_owned(),
            "r.ule".to_owned(),
            "rüle".to_owned(),
        ] {
            let mut request = create_request();
            request.id = id;
            assert!(AlertRuleNew::try_from(request).is_err());
        }

        let mut boundary = create_request();
        boundary.name = "界".repeat(NAME_MAX_CHARS);
        boundary.config.classification = "界".repeat(CLASSIFICATION_MAX_CHARS);
        boundary.config.interval_seconds = SECONDS_MAX;
        assert!(AlertRuleNew::try_from(boundary).is_ok());

        for (name, classification, interval_seconds) in [
            (" ".to_owned(), "class".to_owned(), 60),
            ("界".repeat(NAME_MAX_CHARS + 1), "class".to_owned(), 60),
            ("name".to_owned(), " ".to_owned(), 60),
            (
                "name".to_owned(),
                "界".repeat(CLASSIFICATION_MAX_CHARS + 1),
                60,
            ),
            ("name".to_owned(), "class".to_owned(), 90),
        ] {
            let mut request = create_request();
            request.name = name;
            request.config.classification = classification;
            request.config.interval_seconds = interval_seconds;
            assert!(AlertRuleNew::try_from(request).is_err());
        }
    }

    #[test]
    fn cloudwatch_query_rules_are_validated() {
        for period in [1, 5, 10, 20, 30, 60, SECONDS_MAX] {
            let mut request = create_request();
            cloudwatch_mut(&mut request).period = period;
            let warning = severity_mut(&mut request, Severity::Warning);
            warning.evaluation_periods = 1;
            warning.datapoints_to_alarm = Some(1);
            assert!(AlertRuleNew::try_from(request).is_ok());
        }
        for period in [0, 2, 59, 61, SECONDS_MAX - 1, SECONDS_MAX + 60] {
            let mut request = create_request();
            cloudwatch_mut(&mut request).period = period;
            assert!(AlertRuleNew::try_from(request).is_err());
        }

        let mut blank_region = create_request();
        cloudwatch_mut(&mut blank_region).region = " ".to_owned();
        let mut blank_namespace = create_request();
        cloudwatch_mut(&mut blank_namespace).namespace = " ".to_owned();
        let mut blank_metric = create_request();
        cloudwatch_mut(&mut blank_metric).metric_name = " ".to_owned();
        for request in [blank_region, blank_namespace, blank_metric] {
            assert!(AlertRuleNew::try_from(request).is_err());
        }

        let mut dimensions = create_request();
        cloudwatch_mut(&mut dimensions).dimensions = (0..=DIMENSIONS_MAX)
            .map(|index| (format!("name-{index}"), format!("value-{index}")))
            .collect();
        assert!(AlertRuleNew::try_from(dimensions).is_err());
        for (name, value) in [(" ", "value"), ("name", " ")] {
            let mut request = create_request();
            cloudwatch_mut(&mut request)
                .dimensions
                .insert(name.to_owned(), value.to_owned());
            assert!(AlertRuleNew::try_from(request).is_err());
        }

        for end_offset_seconds in [0, 3600] {
            let mut request = create_request();
            cloudwatch_mut(&mut request).end_offset_seconds = end_offset_seconds;
            assert!(AlertRuleNew::try_from(request).is_ok());
        }
        let mut request = create_request();
        cloudwatch_mut(&mut request).end_offset_seconds = 3601;
        let error = AlertRuleNew::try_from(request).unwrap_err();
        assert!(format!("{error:?}").contains("config.query.end_offset_seconds"));
    }

    #[test]
    fn severity_threshold_rules_and_window_limit_are_validated() {
        let mut empty = create_request();
        empty.config.thresholds.severities = BTreeMap::new();
        assert!(AlertRuleNew::try_from(empty).is_err());

        for threshold in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let mut request = create_request();
            severity_mut(&mut request, Severity::Critical).threshold = threshold;
            assert!(AlertRuleNew::try_from(request).is_err());
        }

        let mut zero = create_request();
        severity_mut(&mut zero, Severity::Critical).evaluation_periods = 0;
        assert!(AlertRuleNew::try_from(zero).is_err());

        for datapoints in [Some(0), Some(2)] {
            let mut request = create_request();
            severity_mut(&mut request, Severity::Critical).datapoints_to_alarm = datapoints;
            assert!(AlertRuleNew::try_from(request).is_err());
        }

        let mut blank = create_request();
        severity_mut(&mut blank, Severity::Critical).description = " ".to_owned();
        assert!(AlertRuleNew::try_from(blank).is_err());

        let mut too_long = create_request();
        severity_mut(&mut too_long, Severity::Critical).evaluation_periods = 1441;
        let error = AlertRuleNew::try_from(too_long).unwrap_err();
        assert!(format!("{error:?}")
            .contains("config.thresholds.severities.critical.evaluation_periods"));

        let mut overflow = create_request();
        severity_mut(&mut overflow, Severity::Critical).evaluation_periods = u32::MAX;
        assert!(AlertRuleNew::try_from(overflow).is_err());
    }

    #[test]
    fn update_requires_a_field_and_validates_whole_config() {
        assert!(AlertRuleUpdate::try_from(api::AlertRuleUpdateRequest {
            name: None,
            config: None,
            status: None,
        })
        .is_err());
        assert!(AlertRuleUpdate::try_from(api::AlertRuleUpdateRequest {
            name: None,
            config: None,
            status: Some(AlertRuleStatus::Inactive),
        })
        .is_ok());

        let mut invalid_config = config_value();
        invalid_config.interval_seconds = 90;
        assert!(AlertRuleUpdate::try_from(api::AlertRuleUpdateRequest {
            name: None,
            config: Some(invalid_config),
            status: None,
        })
        .is_err());
    }

    #[test]
    fn domain_storage_and_api_conversions_are_infallible() {
        let new = AlertRuleNew::try_from(create_request()).unwrap();
        let stored_new = storage::AlertRuleNew::from(new);
        assert_eq!(stored_new.status, AlertRuleStatus::Active);
        assert_eq!(
            stored_new.config.inner().query.datasource().to_string(),
            "cloudwatch"
        );

        let timestamp = datetime!(2026-09-15 09:45:00);
        let response = api::AlertRuleResponse::from(AlertRule::from(storage::AlertRule {
            id: stored_new.id,
            name: stored_new.name,
            config: stored_new.config,
            status: stored_new.status,
            next_run_at: timestamp,
            last_evaluated_at: Some(timestamp),
            created_at: timestamp,
            last_updated_at: timestamp,
        }));
        assert_eq!(response.status, AlertRuleStatus::Active);
        assert_eq!(response.config.classification, "kafka-alerts");
    }
}

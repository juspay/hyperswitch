use api_models::observability::alert_manager::alert_rule_configurations as api;
use diesel_models::observability::alert_manager::alert_rule_configurations as storage;
use error_stack::report;
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::errors::{ObservabilityApiResult, ObservabilityError};

const ID_MAX_CHARS: usize = 64;
const NAME_MAX_CHARS: usize = 255;
const CLASSIFICATION_MAX_CHARS: usize = 64;
const DIMENSIONS_MAX: usize = 30;
const SECONDS_MAX: u32 = 86_400;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "version", rename_all = "snake_case")]
pub enum AlertRuleQuery {
    V1(api::CloudwatchQueryV1),
}

impl AlertRuleQuery {
    pub fn inner(&self) -> &api::CloudwatchQueryV1 {
        match self {
            Self::V1(query) => query,
        }
    }

    pub fn into_inner(self) -> api::CloudwatchQueryV1 {
        match self {
            Self::V1(query) => query,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "version", rename_all = "snake_case")]
pub enum AlertRuleThresholds {
    V1(api::AlertRuleThresholdsV1),
}

impl AlertRuleThresholds {
    pub fn inner(&self) -> &api::AlertRuleThresholdsV1 {
        match self {
            Self::V1(thresholds) => thresholds,
        }
    }

    pub fn into_inner(self) -> api::AlertRuleThresholdsV1 {
        match self {
            Self::V1(thresholds) => thresholds,
        }
    }
}

#[derive(Clone, Debug)]
pub struct AlertRuleConfigurationNew {
    pub id: String,
    pub name: String,
    pub classification: String,
    pub datasource: api::AlertDatasource,
    pub query: AlertRuleQuery,
    pub thresholds: AlertRuleThresholds,
    pub interval_seconds: u32,
    pub is_enabled: bool,
}

#[derive(Clone, Debug)]
pub struct AlertRuleConfiguration {
    pub id: String,
    pub name: String,
    pub classification: String,
    pub datasource: api::AlertDatasource,
    pub query: AlertRuleQuery,
    pub thresholds: AlertRuleThresholds,
    pub interval_seconds: u32,
    pub is_enabled: bool,
    pub next_run_at: PrimitiveDateTime,
    pub last_evaluated_at: Option<PrimitiveDateTime>,
    pub created_at: PrimitiveDateTime,
    pub last_updated_at: PrimitiveDateTime,
}

#[derive(Clone, Debug)]
pub enum AlertRuleConfigurationUpdate {
    Update {
        name: Option<String>,
        classification: Option<String>,
        query: Option<AlertRuleQuery>,
        thresholds: Option<AlertRuleThresholds>,
        interval_seconds: Option<u32>,
        is_enabled: Option<bool>,
    },
    NextRun {
        next_run_at: PrimitiveDateTime,
    },
    Evaluated {
        last_evaluated_at: PrimitiveDateTime,
    },
}

#[derive(Clone, Debug)]
pub struct AlertRuleConfigurationListing {
    pub rules: Vec<AlertRuleConfiguration>,
    pub unreadable_ids: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum AlertRuleConfigurationConversionError {
    #[error("Failed to serialize {field} for alert rule configuration {rule_id}")]
    Serialize {
        rule_id: String,
        field: &'static str,
        #[source]
        source: serde_json::Error,
    },
    #[error("Failed to deserialize {field} for alert rule configuration {rule_id}")]
    Deserialize {
        rule_id: String,
        field: &'static str,
        #[source]
        source: serde_json::Error,
    },
    #[error("Unknown datasource {datasource} for alert rule configuration {rule_id}")]
    Datasource { rule_id: String, datasource: String },
    #[error(
        "{field} is outside the supported integer range for alert rule configuration {rule_id}"
    )]
    IntegerRange {
        rule_id: String,
        field: &'static str,
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
            "interval_seconds must be a whole number of minutes from 60 to 86400".to_owned(),
        ));
    }
    Ok(())
}

fn validate_query(query: &api::CloudwatchQueryV1) -> ObservabilityApiResult<()> {
    required_text("query.region", &query.region)?;
    required_text("query.namespace", &query.namespace)?;
    required_text("query.metric_name", &query.metric_name)?;
    if query.dimensions.len() > DIMENSIONS_MAX {
        return Err(invalid(format!(
            "query.dimensions must contain at most {DIMENSIONS_MAX} entries"
        )));
    }
    for (name, value) in &query.dimensions {
        required_text("query.dimensions name", name)?;
        required_text(&format!("query.dimensions.{name}"), value)?;
    }
    if !matches!(query.period, 1 | 5 | 10 | 20 | 30)
        && (query.period == 0 || query.period > SECONDS_MAX || query.period % 60 != 0)
    {
        return Err(invalid(
            "query.period must be 1, 5, 10, 20, 30, or a multiple of 60 up to 86400".to_owned(),
        ));
    }
    if query.end_offset_seconds > 3600 {
        return Err(invalid(
            "query.end_offset_seconds must be from 0 to 3600".to_owned(),
        ));
    }
    Ok(())
}

fn validate_thresholds(thresholds: &api::AlertRuleThresholdsV1) -> ObservabilityApiResult<()> {
    if thresholds.severities.is_empty() {
        return Err(invalid(
            "thresholds.severities must contain at least one severity".to_owned(),
        ));
    }
    for (severity, threshold) in &thresholds.severities {
        let field = format!("thresholds.severities.{severity}");
        if !threshold.threshold.is_finite() {
            return Err(invalid(format!("{field}.threshold must be finite")));
        }
        if threshold.evaluation_periods == 0 {
            return Err(invalid(format!(
                "{field}.evaluation_periods must be at least 1"
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

impl TryFrom<api::AlertRuleConfigurationCreateRequest> for AlertRuleConfigurationNew {
    type Error = error_stack::Report<ObservabilityError>;

    fn try_from(request: api::AlertRuleConfigurationCreateRequest) -> Result<Self, Self::Error> {
        validate_id(&request.id)?;
        bounded_text("name", &request.name, NAME_MAX_CHARS)?;
        bounded_text(
            "classification",
            &request.classification,
            CLASSIFICATION_MAX_CHARS,
        )?;
        validate_interval_seconds(request.interval_seconds)?;
        validate_query(&request.query)?;
        validate_thresholds(&request.thresholds)?;

        Ok(Self {
            id: request.id,
            name: request.name,
            classification: request.classification,
            datasource: request.datasource,
            query: AlertRuleQuery::V1(request.query),
            thresholds: AlertRuleThresholds::V1(request.thresholds),
            interval_seconds: request.interval_seconds,
            is_enabled: request.is_enabled.unwrap_or(true),
        })
    }
}

impl TryFrom<api::AlertRuleConfigurationUpdateRequest> for AlertRuleConfigurationUpdate {
    type Error = error_stack::Report<ObservabilityError>;

    fn try_from(request: api::AlertRuleConfigurationUpdateRequest) -> Result<Self, Self::Error> {
        if request.name.is_none()
            && request.classification.is_none()
            && request.query.is_none()
            && request.thresholds.is_none()
            && request.interval_seconds.is_none()
            && request.is_enabled.is_none()
        {
            return Err(invalid("at least one field must be provided".to_owned()));
        }
        if let Some(name) = request.name.as_deref() {
            bounded_text("name", name, NAME_MAX_CHARS)?;
        }
        if let Some(classification) = request.classification.as_deref() {
            bounded_text("classification", classification, CLASSIFICATION_MAX_CHARS)?;
        }
        if let Some(query) = request.query.as_ref() {
            validate_query(query)?;
        }
        if let Some(thresholds) = request.thresholds.as_ref() {
            validate_thresholds(thresholds)?;
        }
        if let Some(interval_seconds) = request.interval_seconds {
            validate_interval_seconds(interval_seconds)?;
        }

        Ok(Self::Update {
            name: request.name,
            classification: request.classification,
            query: request.query.map(AlertRuleQuery::V1),
            thresholds: request.thresholds.map(AlertRuleThresholds::V1),
            interval_seconds: request.interval_seconds,
            is_enabled: request.is_enabled,
        })
    }
}

fn serialize<T: Serialize>(
    rule_id: &str,
    field: &'static str,
    value: T,
) -> Result<serde_json::Value, AlertRuleConfigurationConversionError> {
    serde_json::to_value(value).map_err(|source| AlertRuleConfigurationConversionError::Serialize {
        rule_id: rule_id.to_owned(),
        field,
        source,
    })
}

fn deserialize<T: for<'de> Deserialize<'de>>(
    rule_id: &str,
    field: &'static str,
    value: serde_json::Value,
) -> Result<T, AlertRuleConfigurationConversionError> {
    serde_json::from_value(value).map_err(|source| {
        AlertRuleConfigurationConversionError::Deserialize {
            rule_id: rule_id.to_owned(),
            field,
            source,
        }
    })
}

fn to_i32(
    rule_id: &str,
    field: &'static str,
    value: u32,
) -> Result<i32, AlertRuleConfigurationConversionError> {
    i32::try_from(value).map_err(|_| AlertRuleConfigurationConversionError::IntegerRange {
        rule_id: rule_id.to_owned(),
        field,
    })
}

fn to_u32(
    rule_id: &str,
    field: &'static str,
    value: i32,
) -> Result<u32, AlertRuleConfigurationConversionError> {
    u32::try_from(value).map_err(|_| AlertRuleConfigurationConversionError::IntegerRange {
        rule_id: rule_id.to_owned(),
        field,
    })
}

impl TryFrom<AlertRuleConfigurationNew> for storage::AlertRuleConfigurationNew {
    type Error = AlertRuleConfigurationConversionError;

    fn try_from(new: AlertRuleConfigurationNew) -> Result<Self, Self::Error> {
        let interval_seconds = to_i32(&new.id, "interval_seconds", new.interval_seconds)?;
        let query = serialize(&new.id, "query", new.query)?;
        let thresholds = serialize(&new.id, "thresholds", new.thresholds)?;
        Ok(Self {
            id: new.id,
            name: new.name,
            classification: new.classification,
            datasource: new.datasource.to_string(),
            query,
            thresholds,
            interval_seconds,
            is_enabled: new.is_enabled,
        })
    }
}

impl AlertRuleConfigurationUpdate {
    pub fn into_storage(
        self,
        id: &str,
    ) -> Result<storage::AlertRuleConfigurationUpdate, AlertRuleConfigurationConversionError> {
        match self {
            Self::Update {
                name,
                classification,
                query,
                thresholds,
                interval_seconds,
                is_enabled,
            } => Ok(storage::AlertRuleConfigurationUpdate::Update {
                name,
                classification,
                query: query
                    .map(|query| serialize(id, "query", query))
                    .transpose()?,
                thresholds: thresholds
                    .map(|thresholds| serialize(id, "thresholds", thresholds))
                    .transpose()?,
                interval_seconds: interval_seconds
                    .map(|value| to_i32(id, "interval_seconds", value))
                    .transpose()?,
                is_enabled,
            }),
            Self::NextRun { next_run_at } => {
                Ok(storage::AlertRuleConfigurationUpdate::NextRun { next_run_at })
            }
            Self::Evaluated { last_evaluated_at } => {
                Ok(storage::AlertRuleConfigurationUpdate::Evaluated { last_evaluated_at })
            }
        }
    }
}

impl TryFrom<storage::AlertRuleConfiguration> for AlertRuleConfiguration {
    type Error = AlertRuleConfigurationConversionError;

    fn try_from(row: storage::AlertRuleConfiguration) -> Result<Self, Self::Error> {
        let datasource = row.datasource.parse().map_err(|_| {
            AlertRuleConfigurationConversionError::Datasource {
                rule_id: row.id.clone(),
                datasource: row.datasource.clone(),
            }
        })?;
        let query = deserialize(&row.id, "query", row.query)?;
        let thresholds = deserialize(&row.id, "thresholds", row.thresholds)?;
        let interval_seconds = to_u32(&row.id, "interval_seconds", row.interval_seconds)?;
        Ok(Self {
            id: row.id,
            name: row.name,
            classification: row.classification,
            datasource,
            query,
            thresholds,
            interval_seconds,
            is_enabled: row.is_enabled,
            next_run_at: row.next_run_at,
            last_evaluated_at: row.last_evaluated_at,
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
            classification: rule.classification,
            datasource: rule.datasource,
            query: rule.query.into_inner(),
            thresholds: rule.thresholds.into_inner(),
            interval_seconds: rule.interval_seconds,
            is_enabled: rule.is_enabled,
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

    use serde_json::{json, Value};
    use time::macros::datetime;

    use super::*;

    fn query_json() -> Value {
        json!({
            "version": "v1",
            "region": "us-east-1",
            "namespace": "AWS/EC2",
            "metric_name": "CPUUtilization",
            "dimensions": {"InstanceId": "i-053c2d51a2fc3535d"},
            "statistic": "Average",
            "period": 60,
            "end_offset_seconds": 180
        })
    }

    fn thresholds_json() -> Value {
        json!({
            "version": "v1",
            "severities": {
                "sev1": {
                    "threshold": 80.0,
                    "comparison_operator": "GreaterThanOrEqualToThreshold",
                    "evaluation_periods": 1,
                    "treat_missing_data": "breaching",
                    "description": "SEV1: Kafka broker 00 CPU utilization is above 80%."
                },
                "sev2": {
                    "threshold": 70.0,
                    "comparison_operator": "GreaterThanOrEqualToThreshold",
                    "evaluation_periods": 3,
                    "datapoints_to_alarm": 2,
                    "treat_missing_data": "missing",
                    "description": "SEV2: Kafka broker 00 CPU utilization is above 70%."
                }
            }
        })
    }

    fn plain_query_json() -> Value {
        let mut value = query_json();
        value.as_object_mut().unwrap().remove("version");
        value
    }

    fn plain_thresholds_json() -> Value {
        let mut value = thresholds_json();
        value.as_object_mut().unwrap().remove("version");
        value
    }

    fn query_value() -> api::CloudwatchQueryV1 {
        serde_json::from_value(plain_query_json()).unwrap()
    }

    fn thresholds_value() -> api::AlertRuleThresholdsV1 {
        serde_json::from_value(plain_thresholds_json()).unwrap()
    }

    fn create_request() -> api::AlertRuleConfigurationCreateRequest {
        api::AlertRuleConfigurationCreateRequest {
            id: "kafka-broker-00-cpu".to_owned(),
            name: "Kafka broker CPU".to_owned(),
            classification: "kafka-alerts".to_owned(),
            datasource: api::AlertDatasource::Cloudwatch,
            query: query_value(),
            thresholds: thresholds_value(),
            interval_seconds: 60,
            is_enabled: None,
        }
    }

    fn severity_mut(
        request: &mut api::AlertRuleConfigurationCreateRequest,
        severity: api::Severity,
    ) -> &mut api::SeverityThresholdV1 {
        request.thresholds.severities.get_mut(&severity).unwrap()
    }

    #[test]
    fn stored_json_round_trips_with_version_tags() {
        let parsed_query: AlertRuleQuery = serde_json::from_value(query_json()).unwrap();
        let parsed_thresholds: AlertRuleThresholds =
            serde_json::from_value(thresholds_json()).unwrap();

        assert_eq!(serde_json::to_value(&parsed_query).unwrap(), query_json());
        assert_eq!(
            serde_json::to_value(&parsed_thresholds).unwrap(),
            thresholds_json()
        );
        assert_eq!(parsed_query.inner(), &query_value());
        assert_eq!(parsed_query.clone().into_inner(), query_value());
        assert_eq!(parsed_thresholds.inner(), &thresholds_value());
        assert_eq!(parsed_thresholds.clone().into_inner(), thresholds_value());
    }

    #[test]
    fn missing_end_offset_defaults_to_180_and_is_stored() {
        let mut query = plain_query_json();
        query.as_object_mut().unwrap().remove("end_offset_seconds");
        let mut request = create_request();
        request.query = serde_json::from_value(query).unwrap();
        assert_eq!(request.query.end_offset_seconds, 180);

        let stored = storage::AlertRuleConfigurationNew::try_from(
            AlertRuleConfigurationNew::try_from(request).unwrap(),
        )
        .unwrap();
        assert_eq!(stored.query["end_offset_seconds"], 180);
    }

    #[test]
    fn query_end_offset_boundaries_are_validated() {
        for end_offset_seconds in [0, 3600] {
            let mut request = create_request();
            request.query.end_offset_seconds = end_offset_seconds;
            assert!(AlertRuleConfigurationNew::try_from(request).is_ok());
        }
        let mut request = create_request();
        request.query.end_offset_seconds = 3601;
        let error = AlertRuleConfigurationNew::try_from(request).unwrap_err();
        assert!(matches!(
            error.current_context(),
            ObservabilityError::InvalidRequest
        ));
        assert!(format!("{error:?}").contains("query.end_offset_seconds"));
    }

    #[test]
    fn unsupported_stored_and_plain_values_fail_to_parse() {
        let mut query_version = query_json();
        query_version["version"] = json!("v2");
        assert!(serde_json::from_value::<AlertRuleQuery>(query_version).is_err());

        let mut thresholds_version = thresholds_json();
        thresholds_version["version"] = json!("v2");
        assert!(serde_json::from_value::<AlertRuleThresholds>(thresholds_version).is_err());

        let mut severity = plain_thresholds_json();
        severity["severities"]["sev4"] = severity["severities"]["sev1"].clone();
        assert!(serde_json::from_value::<api::AlertRuleThresholdsV1>(severity).is_err());

        let mut operator = plain_thresholds_json();
        operator["severities"]["sev1"]["comparison_operator"] = json!("EqualToThreshold");
        assert!(serde_json::from_value::<api::AlertRuleThresholdsV1>(operator).is_err());

        let mut missing_data = plain_thresholds_json();
        missing_data["severities"]["sev1"]["treat_missing_data"] = json!("ignore");
        assert!(serde_json::from_value::<api::AlertRuleThresholdsV1>(missing_data).is_err());

        let mut statistic = plain_query_json();
        statistic["statistic"] = json!("p99");
        assert!(serde_json::from_value::<api::CloudwatchQueryV1>(statistic).is_err());
    }

    #[test]
    fn wire_names_and_defaults_match_the_contract() {
        assert_eq!(
            serde_json::to_value(api::AlertDatasource::Cloudwatch).unwrap(),
            "cloudwatch"
        );
        assert_eq!(
            "cloudwatch".parse::<api::AlertDatasource>().unwrap(),
            api::AlertDatasource::Cloudwatch
        );
        for (operator, expected) in [
            (
                api::ComparisonOperator::GreaterThanThreshold,
                "GreaterThanThreshold",
            ),
            (
                api::ComparisonOperator::GreaterThanOrEqualToThreshold,
                "GreaterThanOrEqualToThreshold",
            ),
            (
                api::ComparisonOperator::LessThanThreshold,
                "LessThanThreshold",
            ),
            (
                api::ComparisonOperator::LessThanOrEqualToThreshold,
                "LessThanOrEqualToThreshold",
            ),
        ] {
            assert_eq!(serde_json::to_value(operator).unwrap(), expected);
            assert_eq!(operator.to_string(), expected);
        }
        for (missing_data, expected) in [
            (api::TreatMissingData::Breaching, "breaching"),
            (api::TreatMissingData::NotBreaching, "notBreaching"),
            (api::TreatMissingData::Missing, "missing"),
        ] {
            assert_eq!(serde_json::to_value(missing_data).unwrap(), expected);
            assert_eq!(missing_data.to_string(), expected);
        }

        let mut threshold = plain_thresholds_json();
        threshold["severities"]["sev1"]
            .as_object_mut()
            .unwrap()
            .remove("treat_missing_data");
        threshold["severities"]["sev1"]
            .as_object_mut()
            .unwrap()
            .remove("datapoints_to_alarm");
        let parsed: api::AlertRuleThresholdsV1 = serde_json::from_value(threshold).unwrap();
        let sev1 = &parsed.severities[&api::Severity::Sev1];
        assert_eq!(sev1.treat_missing_data, api::TreatMissingData::Missing);
        assert_eq!(sev1.datapoints_to_alarm, None);
        assert!(serde_json::to_value(parsed).unwrap()["severities"]["sev1"]
            .get("datapoints_to_alarm")
            .is_none());

        let mut query = plain_query_json();
        query.as_object_mut().unwrap().remove("dimensions");
        assert!(serde_json::from_value::<api::CloudwatchQueryV1>(query)
            .unwrap()
            .dimensions
            .is_empty());
    }

    #[test]
    fn unknown_api_fields_are_rejected() {
        let mut request = json!({
            "id": "rule-1",
            "name": "name",
            "classification": "class",
            "datasource": "cloudwatch",
            "query": plain_query_json(),
            "thresholds": plain_thresholds_json(),
            "interval_seconds": 60
        });
        request["extra"] = json!(true);
        assert!(
            serde_json::from_value::<api::AlertRuleConfigurationCreateRequest>(request).is_err()
        );

        let mut query = plain_query_json();
        query["extra"] = json!(true);
        assert!(serde_json::from_value::<api::CloudwatchQueryV1>(query).is_err());

        let mut thresholds = plain_thresholds_json();
        thresholds["extra"] = json!(true);
        assert!(serde_json::from_value::<api::AlertRuleThresholdsV1>(thresholds).is_err());

        let mut severity = plain_thresholds_json();
        severity["severities"]["sev1"]["extra"] = json!(true);
        assert!(serde_json::from_value::<api::AlertRuleThresholdsV1>(severity).is_err());
    }

    #[test]
    fn id_boundaries_and_characters_are_validated() {
        for id in ["a".to_owned(), "1".repeat(ID_MAX_CHARS)] {
            let mut request = create_request();
            request.id = id;
            assert!(AlertRuleConfigurationNew::try_from(request).is_ok());
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
            assert!(AlertRuleConfigurationNew::try_from(request).is_err());
        }
    }

    #[test]
    fn name_and_classification_boundaries_are_validated() {
        let mut request = create_request();
        request.name = "界".repeat(NAME_MAX_CHARS);
        request.classification = "界".repeat(CLASSIFICATION_MAX_CHARS);
        assert!(AlertRuleConfigurationNew::try_from(request).is_ok());

        for name in [" \n".to_owned(), "界".repeat(NAME_MAX_CHARS + 1)] {
            let mut request = create_request();
            request.name = name;
            assert!(AlertRuleConfigurationNew::try_from(request).is_err());
        }
        for classification in [" \t".to_owned(), "界".repeat(CLASSIFICATION_MAX_CHARS + 1)] {
            let mut request = create_request();
            request.classification = classification;
            assert!(AlertRuleConfigurationNew::try_from(request).is_err());
        }
    }

    #[test]
    fn interval_boundaries_are_validated() {
        for interval_seconds in [60, 120, SECONDS_MAX] {
            let mut request = create_request();
            request.interval_seconds = interval_seconds;
            assert!(AlertRuleConfigurationNew::try_from(request).is_ok());
        }
        for interval_seconds in [0, 59, 61, 90, SECONDS_MAX + 60] {
            let mut request = create_request();
            request.interval_seconds = interval_seconds;
            assert!(AlertRuleConfigurationNew::try_from(request).is_err());
        }
    }

    #[test]
    fn query_text_and_dimension_boundaries_are_validated() {
        let mut blank_region = create_request();
        blank_region.query.region = "  ".to_owned();
        let mut blank_namespace = create_request();
        blank_namespace.query.namespace = "\n".to_owned();
        let mut blank_metric = create_request();
        blank_metric.query.metric_name = "\t".to_owned();
        for request in [blank_region, blank_namespace, blank_metric] {
            assert!(AlertRuleConfigurationNew::try_from(request).is_err());
        }

        let mut boundary = create_request();
        boundary.query.dimensions = (0..DIMENSIONS_MAX)
            .map(|index| (format!("name-{index}"), format!("value-{index}")))
            .collect();
        assert!(AlertRuleConfigurationNew::try_from(boundary).is_ok());

        let mut too_many = create_request();
        too_many.query.dimensions = (0..=DIMENSIONS_MAX)
            .map(|index| (format!("name-{index}"), format!("value-{index}")))
            .collect();
        assert!(AlertRuleConfigurationNew::try_from(too_many).is_err());

        let mut blank_name = create_request();
        blank_name
            .query
            .dimensions
            .insert(" ".to_owned(), "value".to_owned());
        assert!(AlertRuleConfigurationNew::try_from(blank_name).is_err());

        let mut blank_value = create_request();
        blank_value
            .query
            .dimensions
            .insert("name".to_owned(), " ".to_owned());
        assert!(AlertRuleConfigurationNew::try_from(blank_value).is_err());
    }

    #[test]
    fn query_period_boundaries_are_validated() {
        for period in [1, 5, 10, 20, 30, 60, SECONDS_MAX] {
            let mut request = create_request();
            request.query.period = period;
            assert!(AlertRuleConfigurationNew::try_from(request).is_ok());
        }
        for period in [0, 2, 59, 61, SECONDS_MAX - 1, SECONDS_MAX + 60] {
            let mut request = create_request();
            request.query.period = period;
            assert!(AlertRuleConfigurationNew::try_from(request).is_err());
        }
    }

    #[test]
    fn threshold_boundaries_are_validated() {
        let mut empty = create_request();
        empty.thresholds.severities = BTreeMap::new();
        assert!(AlertRuleConfigurationNew::try_from(empty).is_err());

        for threshold in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let mut request = create_request();
            severity_mut(&mut request, api::Severity::Sev1).threshold = threshold;
            assert!(AlertRuleConfigurationNew::try_from(request).is_err());
        }

        let mut zero_periods = create_request();
        severity_mut(&mut zero_periods, api::Severity::Sev1).evaluation_periods = 0;
        assert!(AlertRuleConfigurationNew::try_from(zero_periods).is_err());

        for datapoints in [Some(0), Some(2)] {
            let mut request = create_request();
            severity_mut(&mut request, api::Severity::Sev1).datapoints_to_alarm = datapoints;
            assert!(AlertRuleConfigurationNew::try_from(request).is_err());
        }

        let mut boundary = create_request();
        severity_mut(&mut boundary, api::Severity::Sev2).datapoints_to_alarm = Some(3);
        assert!(AlertRuleConfigurationNew::try_from(boundary).is_ok());

        let mut blank_description = create_request();
        severity_mut(&mut blank_description, api::Severity::Sev1).description = " ".to_owned();
        assert!(AlertRuleConfigurationNew::try_from(blank_description).is_err());
    }

    #[test]
    fn update_requires_and_validates_present_fields() {
        assert!(
            AlertRuleConfigurationUpdate::try_from(api::AlertRuleConfigurationUpdateRequest {
                name: None,
                classification: None,
                query: None,
                thresholds: None,
                interval_seconds: None,
                is_enabled: None,
            })
            .is_err()
        );
        assert!(
            AlertRuleConfigurationUpdate::try_from(api::AlertRuleConfigurationUpdateRequest {
                name: None,
                classification: None,
                query: None,
                thresholds: None,
                interval_seconds: None,
                is_enabled: Some(false),
            })
            .is_ok()
        );

        let invalid_requests = [
            api::AlertRuleConfigurationUpdateRequest {
                name: Some(" ".to_owned()),
                classification: None,
                query: None,
                thresholds: None,
                interval_seconds: None,
                is_enabled: None,
            },
            api::AlertRuleConfigurationUpdateRequest {
                name: None,
                classification: Some(" ".to_owned()),
                query: None,
                thresholds: None,
                interval_seconds: None,
                is_enabled: None,
            },
            api::AlertRuleConfigurationUpdateRequest {
                name: None,
                classification: None,
                query: Some(api::CloudwatchQueryV1 {
                    period: 0,
                    ..query_value()
                }),
                thresholds: None,
                interval_seconds: None,
                is_enabled: None,
            },
            api::AlertRuleConfigurationUpdateRequest {
                name: None,
                classification: None,
                query: None,
                thresholds: Some(api::AlertRuleThresholdsV1 {
                    severities: BTreeMap::new(),
                }),
                interval_seconds: None,
                is_enabled: None,
            },
            api::AlertRuleConfigurationUpdateRequest {
                name: None,
                classification: None,
                query: None,
                thresholds: None,
                interval_seconds: Some(90),
                is_enabled: None,
            },
        ];
        for request in invalid_requests {
            assert!(AlertRuleConfigurationUpdate::try_from(request).is_err());
        }
    }

    #[test]
    fn storage_and_response_conversions_preserve_fields_and_hide_versions() {
        let new = AlertRuleConfigurationNew::try_from(create_request()).unwrap();
        let stored_new = storage::AlertRuleConfigurationNew::try_from(new).unwrap();
        assert_eq!(stored_new.id, "kafka-broker-00-cpu");
        assert_eq!(stored_new.datasource, "cloudwatch");
        assert_eq!(stored_new.query["version"], "v1");
        assert_eq!(stored_new.thresholds["version"], "v1");

        let timestamp = datetime!(2026-09-15 09:45:00);
        let stored = storage::AlertRuleConfiguration {
            id: stored_new.id,
            name: stored_new.name,
            classification: stored_new.classification,
            datasource: stored_new.datasource,
            query: stored_new.query,
            thresholds: stored_new.thresholds,
            interval_seconds: stored_new.interval_seconds,
            is_enabled: stored_new.is_enabled,
            next_run_at: timestamp,
            last_evaluated_at: Some(timestamp),
            created_at: timestamp,
            last_updated_at: timestamp,
        };
        let response = api::AlertRuleConfigurationResponse::from(
            AlertRuleConfiguration::try_from(stored).unwrap(),
        );
        let serialized = serde_json::to_value(response).unwrap();
        assert!(serialized["query"].get("version").is_none());
        assert!(serialized["thresholds"].get("version").is_none());
        assert_eq!(serialized["datasource"], "cloudwatch");
        assert_eq!(serialized["last_evaluated_at"], "2026-09-15T09:45:00.000Z");
    }

    #[test]
    fn update_conversion_carries_rule_id_and_field_context() {
        let update = AlertRuleConfigurationUpdate::Update {
            name: None,
            classification: None,
            query: Some(AlertRuleQuery::V1(query_value())),
            thresholds: Some(AlertRuleThresholds::V1(thresholds_value())),
            interval_seconds: Some(60),
            is_enabled: Some(false),
        };
        let stored = update.into_storage("kafka-broker-00-cpu").unwrap();
        assert!(matches!(
            stored,
            storage::AlertRuleConfigurationUpdate::Update {
                query: Some(_),
                thresholds: Some(_),
                interval_seconds: Some(60),
                is_enabled: Some(false),
                ..
            }
        ));
    }

    #[test]
    fn malformed_storage_values_and_integer_overflow_are_rejected() {
        let timestamp = datetime!(2026-09-15 09:45:00);
        let stored = |datasource: &str, query: Value, thresholds: Value, interval_seconds: i32| {
            storage::AlertRuleConfiguration {
                id: "rule-bad".to_owned(),
                name: "bad".to_owned(),
                classification: "bad".to_owned(),
                datasource: datasource.to_owned(),
                query,
                thresholds,
                interval_seconds,
                is_enabled: true,
                next_run_at: timestamp,
                last_evaluated_at: None,
                created_at: timestamp,
                last_updated_at: timestamp,
            }
        };

        assert!(AlertRuleConfiguration::try_from(stored(
            "prometheus",
            query_json(),
            thresholds_json(),
            60,
        ))
        .is_err());
        assert!(AlertRuleConfiguration::try_from(stored(
            "cloudwatch",
            json!({"version": "v2"}),
            thresholds_json(),
            60,
        ))
        .is_err());
        assert!(AlertRuleConfiguration::try_from(stored(
            "cloudwatch",
            query_json(),
            json!({"version": "v2"}),
            60,
        ))
        .is_err());
        assert!(AlertRuleConfiguration::try_from(stored(
            "cloudwatch",
            query_json(),
            thresholds_json(),
            -1,
        ))
        .is_err());

        let mut new = AlertRuleConfigurationNew::try_from(create_request()).unwrap();
        new.interval_seconds = u32::MAX;
        assert!(storage::AlertRuleConfigurationNew::try_from(new).is_err());
    }
}

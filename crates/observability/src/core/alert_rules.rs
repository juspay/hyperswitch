//! Per-request logic for alert rules.

use api_models::observability::alert_manager::alert_rules::{
    AlertRuleCreateRequest, AlertRuleDeleteResponse, AlertRuleListResponse, AlertRuleResponse,
    AlertRuleUpdateRequest,
};
use common_types::alert_rules::{AlertQuery, AlertRuleConfig, MetricSource};
use common_utils::date_time;
use error_stack::{report, ResultExt};

use crate::{
    domain_models::alert_rules::{invalid, AlertRuleNew},
    errors::{ObservabilityApiResult, ObservabilityError},
    state::AppState,
};

pub async fn list(state: AppState, _: ()) -> ObservabilityApiResult<AlertRuleListResponse> {
    let rules = state
        .store
        .list_alert_rules()
        .await
        .change_context(ObservabilityError::InternalServerError)
        .attach_printable("Failed to list alert rules")?;

    Ok(AlertRuleListResponse {
        rules: rules.into_iter().map(AlertRuleResponse::from).collect(),
    })
}

pub async fn get(state: AppState, id: String) -> ObservabilityApiResult<AlertRuleResponse> {
    state
        .store
        .find_alert_rule(&id)
        .await
        .change_context(ObservabilityError::InternalServerError)
        .attach_printable("Failed to find alert rule")?
        .map(AlertRuleResponse::from)
        .ok_or_else(|| report!(ObservabilityError::AlertRuleNotFound { id }))
}

/// Create a rule. It is due on the evaluator's next tick.
pub async fn create(
    state: AppState,
    request: AlertRuleCreateRequest,
) -> ObservabilityApiResult<AlertRuleResponse> {
    let AlertRuleCreateRequest {
        id,
        name,
        status,
        alerts_configuration,
    } = request;
    let new = AlertRuleNew::try_from_request(
        id,
        AlertRuleUpdateRequest {
            name,
            status,
            alerts_configuration,
        },
        date_time::now(),
    )?;
    check_against_settings(
        &state,
        &AlertRuleConfig::from(new.alerts_configuration().clone()),
    )?;

    let id = new.id().to_owned();
    state
        .store
        .insert_alert_rule(new)
        .await
        .change_context(ObservabilityError::InternalServerError)
        .attach_printable("Failed to create alert rule")?
        .map(AlertRuleResponse::from)
        .ok_or_else(|| invalid(format!("id `{id}` is already in use")))
}

/// Replace a rule. It is then due on the evaluator's next tick.
pub async fn update(
    state: AppState,
    (id, request): (String, AlertRuleUpdateRequest),
) -> ObservabilityApiResult<AlertRuleResponse> {
    let new = AlertRuleNew::try_from_request(id.clone(), request, date_time::now())?;
    check_against_settings(
        &state,
        &AlertRuleConfig::from(new.alerts_configuration().clone()),
    )?;

    state
        .store
        .update_alert_rule(new)
        .await
        .change_context(ObservabilityError::InternalServerError)
        .attach_printable("Failed to update alert rule")?
        .map(AlertRuleResponse::from)
        .ok_or_else(|| report!(ObservabilityError::AlertRuleNotFound { id }))
}

pub async fn delete(
    state: AppState,
    id: String,
) -> ObservabilityApiResult<AlertRuleDeleteResponse> {
    let deleted = state
        .store
        .delete_alert_rule(&id)
        .await
        .change_context(ObservabilityError::InternalServerError)
        .attach_printable("Failed to delete alert rule")?;

    if deleted {
        Ok(AlertRuleDeleteResponse { ok: true, id })
    } else {
        Err(report!(ObservabilityError::AlertRuleNotFound { id }))
    }
}

/// What the rule's own checks cannot know: whether this deployment can query its datasource and
/// deliver to its destinations. Done on the evaluator's shape, so it checks what will be used.
fn check_against_settings(state: &AppState, rule: &AlertRuleConfig) -> ObservabilityApiResult<()> {
    let source = match rule.query {
        AlertQuery::GrafanaCloudwatch(_) => MetricSource::GrafanaCloudwatch,
    };
    if !state
        .conf
        .alert
        .get_inner()
        .datasources
        .contains_key(&source)
    {
        return Err(invalid(format!(
            "alerts_configuration.query needs `alert.datasources.{source}` to be configured"
        )));
    }

    for (severity, threshold) in &rule.thresholds.severities {
        if state.chat.get(&threshold.destination).is_none() {
            return Err(invalid(format!(
                "alerts_configuration.thresholds.severities.{severity}.destination `{}` is not a \
                 configured chat destination",
                threshold.destination
            )));
        }
    }

    Ok(())
}

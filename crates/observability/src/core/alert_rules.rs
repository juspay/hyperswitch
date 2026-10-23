//! Per-request logic for alert rules.

use api_models::observability::alert_manager::alert_rules::{
    AlertRuleDeleteResponse, AlertRuleListResponse, AlertRuleResponse, AlertRuleUpsertRequest,
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

/// Create or replace a rule. Either way it is due on the evaluator's next tick.
pub async fn upsert(
    state: AppState,
    (id, request): (String, AlertRuleUpsertRequest),
) -> ObservabilityApiResult<AlertRuleResponse> {
    let new = AlertRuleNew::try_from_request(id, request, date_time::now())?;
    check_against_settings(
        &state,
        &AlertRuleConfig::from(new.alerts_configuration().clone()),
    )?;

    let rule = state
        .store
        .upsert_alert_rule(new)
        .await
        .change_context(ObservabilityError::InternalServerError)
        .attach_printable("Failed to store alert rule")?;

    Ok(AlertRuleResponse::from(rule))
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

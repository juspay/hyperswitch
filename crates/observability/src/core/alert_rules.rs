use api_models::observability::alert_manager::alert_rules::{
    AlertRuleCreateRequest, AlertRuleListResponse, AlertRuleResponse, AlertRuleUpdateRequest,
};
use diesel_models::errors::DatabaseError;
use error_stack::{report, ResultExt};

use crate::{
    domain_models::alert_rules::{AlertRuleNew, AlertRuleUpdate},
    errors::{ObservabilityApiResult, ObservabilityError},
    state::AppState,
};

pub async fn create_alert_rule(
    state: AppState,
    request: AlertRuleCreateRequest,
) -> ObservabilityApiResult<AlertRuleResponse> {
    let new = AlertRuleNew::try_from(request)?;
    let rule = match state.store.insert_alert_rule(new).await {
        Ok(rule) => rule,
        Err(error) if matches!(error.current_context(), DatabaseError::UniqueViolation) => {
            return Err(report!(ObservabilityError::InvalidRequest)
                .attach_printable("An alert rule with this id already exists"));
        }
        Err(error) => {
            return Err(error
                .change_context(ObservabilityError::InternalServerError)
                .attach_printable("Failed to insert alert rule"));
        }
    };
    Ok(AlertRuleResponse::from(rule))
}

pub async fn update_alert_rule(
    state: AppState,
    (id, request): (String, AlertRuleUpdateRequest),
) -> ObservabilityApiResult<AlertRuleResponse> {
    let update = AlertRuleUpdate::try_from(request)?;
    let rule = match state.store.update_alert_rule(id, update).await {
        Ok(rule) => rule,
        Err(error) if matches!(error.current_context(), DatabaseError::NotFound) => {
            return Err(report!(ObservabilityError::AlertRuleNotFound));
        }
        Err(error) => {
            return Err(error
                .change_context(ObservabilityError::InternalServerError)
                .attach_printable("Failed to update alert rule"));
        }
    };
    Ok(AlertRuleResponse::from(rule))
}

pub async fn list_alert_rules(
    state: AppState,
    _: (),
) -> ObservabilityApiResult<AlertRuleListResponse> {
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

use api_models::observability::alert_manager::alert_rule_configurations::{
    AlertRuleConfigurationCreateRequest, AlertRuleConfigurationListResponse,
    AlertRuleConfigurationResponse, AlertRuleConfigurationUpdateRequest,
};
use diesel_models::errors::DatabaseError;
use error_stack::{report, ResultExt};

use crate::{
    domain_models::alert_rule_configurations::{
        AlertRuleConfiguration, AlertRuleConfigurationNew, AlertRuleConfigurationUpdate,
    },
    errors::{ObservabilityApiResult, ObservabilityError},
    state::AppState,
};

pub async fn create_alert_rule_configuration(
    state: AppState,
    request: AlertRuleConfigurationCreateRequest,
) -> ObservabilityApiResult<AlertRuleConfigurationResponse> {
    let new = AlertRuleConfigurationNew::try_from(request)?;
    let stored = match state.store.insert_alert_rule_configuration(new).await {
        Ok(stored) => stored,
        Err(error) if matches!(error.current_context(), DatabaseError::UniqueViolation) => {
            return Err(report!(ObservabilityError::InvalidRequest)
                .attach_printable("An alert rule configuration with this name already exists"));
        }
        Err(error) => {
            return Err(error
                .change_context(ObservabilityError::InternalServerError)
                .attach_printable("Failed to insert alert rule configuration"));
        }
    };
    let rule = AlertRuleConfiguration::try_from(stored)
        .change_context(ObservabilityError::InternalServerError)
        .attach_printable("Failed to deserialize stored alert rule configuration")?;
    Ok(AlertRuleConfigurationResponse::from(rule))
}

pub async fn update_alert_rule_configuration(
    state: AppState,
    id: String,
    request: AlertRuleConfigurationUpdateRequest,
) -> ObservabilityApiResult<AlertRuleConfigurationResponse> {
    let update = AlertRuleConfigurationUpdate::try_from(request)?;
    let stored = match state
        .store
        .update_alert_rule_configuration(id, update)
        .await
    {
        Ok(stored) => stored,
        Err(error) if matches!(error.current_context(), DatabaseError::NotFound) => {
            return Err(report!(ObservabilityError::InvalidRequest)
                .attach_printable("Unknown alert rule configuration id"));
        }
        Err(error) => {
            return Err(error
                .change_context(ObservabilityError::InternalServerError)
                .attach_printable("Failed to update alert rule configuration"));
        }
    };
    let rule = AlertRuleConfiguration::try_from(stored)
        .change_context(ObservabilityError::InternalServerError)
        .attach_printable("Failed to deserialize stored alert rule configuration")?;
    Ok(AlertRuleConfigurationResponse::from(rule))
}

pub async fn list_alert_rule_configurations(
    state: AppState,
    _: (),
) -> ObservabilityApiResult<AlertRuleConfigurationListResponse> {
    let rows = state
        .store
        .list_alert_rule_configurations()
        .await
        .change_context(ObservabilityError::InternalServerError)
        .attach_printable("Failed to list alert rule configurations")?;
    let mut rule_configurations = Vec::with_capacity(rows.len());
    let mut unreadable_ids = Vec::new();
    for row in rows {
        let id = row.id.clone();
        match AlertRuleConfiguration::try_from(row) {
            Ok(rule) => rule_configurations.push(AlertRuleConfigurationResponse::from(rule)),
            Err(error) => {
                router_env::logger::error!(rule_id = %id, error = ?error, "Failed to deserialize alert rule configuration");
                unreadable_ids.push(id);
            }
        }
    }
    Ok(AlertRuleConfigurationListResponse {
        rule_configurations,
        unreadable_ids,
    })
}

use api_models::observability::alert_manager::alert_rule_configurations::{
    AlertRuleConfigurationCreateRequest, AlertRuleConfigurationListResponse,
    AlertRuleConfigurationResponse, AlertRuleConfigurationUpdateRequest,
};
use diesel_models::errors::DatabaseError;
use error_stack::{report, ResultExt};

use crate::{
    domain_models::alert_rule_configurations::{
        AlertRuleConfigurationNew, AlertRuleConfigurationUpdate,
    },
    errors::{ObservabilityApiResult, ObservabilityError},
    state::AppState,
};

pub async fn create_alert_rule_configuration(
    state: AppState,
    request: AlertRuleConfigurationCreateRequest,
) -> ObservabilityApiResult<AlertRuleConfigurationResponse> {
    let new = AlertRuleConfigurationNew::try_from(request)?;
    let rule = match state.store.insert_alert_rule_configuration(new).await {
        Ok(rule) => rule,
        Err(error) if matches!(error.current_context(), DatabaseError::UniqueViolation) => {
            return Err(
                report!(ObservabilityError::InvalidRequest).attach_printable(
                    "An alert rule configuration with this id or name already exists",
                ),
            );
        }
        Err(error) => {
            return Err(error
                .change_context(ObservabilityError::InternalServerError)
                .attach_printable("Failed to insert alert rule configuration"));
        }
    };
    Ok(AlertRuleConfigurationResponse::from(rule))
}

pub async fn update_alert_rule_configuration(
    state: AppState,
    (id, request): (String, AlertRuleConfigurationUpdateRequest),
) -> ObservabilityApiResult<AlertRuleConfigurationResponse> {
    let update = AlertRuleConfigurationUpdate::try_from(request)?;
    let rule = match state
        .store
        .update_alert_rule_configuration(id, update)
        .await
    {
        Ok(rule) => rule,
        Err(error) if matches!(error.current_context(), DatabaseError::NotFound) => {
            return Err(report!(ObservabilityError::AlertRuleConfigurationNotFound));
        }
        Err(error) => {
            return Err(error
                .change_context(ObservabilityError::InternalServerError)
                .attach_printable("Failed to update alert rule configuration"));
        }
    };
    Ok(AlertRuleConfigurationResponse::from(rule))
}

pub async fn list_alert_rule_configurations(
    state: AppState,
    _: (),
) -> ObservabilityApiResult<AlertRuleConfigurationListResponse> {
    let listing = state
        .store
        .list_alert_rule_configurations()
        .await
        .change_context(ObservabilityError::InternalServerError)
        .attach_printable("Failed to list alert rule configurations")?;
    Ok(AlertRuleConfigurationListResponse {
        rule_configurations: listing
            .rules
            .into_iter()
            .map(AlertRuleConfigurationResponse::from)
            .collect(),
        unreadable_ids: listing.unreadable_ids,
    })
}

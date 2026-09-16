//! Per-request logic for alert rule enable/disable state.

use api_models::observability::alert_manager::rule_toggles::{
    RuleToggleListResponse, RuleToggleResponse, RuleToggleSetRequest, RuleToggleSetResponse,
};
use error_stack::ResultExt;

use crate::{
    domain_models::rule_toggles::RuleToggleNew,
    errors::{ObservabilityApiResult, ObservabilityError},
    state::AppState,
};

pub async fn list(state: AppState, _: ()) -> ObservabilityApiResult<RuleToggleListResponse> {
    let toggles = state
        .store
        .list_rule_toggles()
        .await
        .change_context(ObservabilityError::InternalServerError)
        .attach_printable("Failed to list rule toggles")?;

    Ok(RuleToggleListResponse {
        toggles: toggles.into_iter().map(RuleToggleResponse::from).collect(),
    })
}

pub async fn set(
    state: AppState,
    (rule_id, request): (String, RuleToggleSetRequest),
) -> ObservabilityApiResult<RuleToggleSetResponse> {
    let new = RuleToggleNew::try_from_request(rule_id, request)?;
    let toggle = state
        .store
        .set_rule_toggle(new)
        .await
        .change_context(ObservabilityError::InternalServerError)
        .attach_printable("Failed to set rule toggle")?;

    Ok(RuleToggleSetResponse {
        ok: true,
        id: toggle.rule_id,
        is_enabled: toggle.is_enabled,
    })
}

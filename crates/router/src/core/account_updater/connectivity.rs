use super::{config::resolve_account_updater_config, connector_config};
use crate::{
    core::{configs::dimension_state, errors::RouterResponse},
    routes::SessionState,
    services::ApplicationResponse,
};

impl common_utils::events::ApiEventMetric for AccountUpdaterConnectivityResponse {}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AccountUpdaterConnectivityResponse {
    pub enabled: bool,
    pub connector_config_built: Option<bool>,
    pub detail: String,
}

pub async fn check_account_updater_connectivity(
    state: SessionState,
) -> RouterResponse<AccountUpdaterConnectivityResponse> {
    let dimensions: dimension_state::DimensionsGlobal = dimension_state::Dimensions::new();

    let response = match resolve_account_updater_config(&state, &dimensions).await {
        Err(error) => AccountUpdaterConnectivityResponse {
            enabled: false,
            connector_config_built: None,
            detail: format!("Account Updater config could not be resolved: {error:?}"),
        },
        Ok(None) => AccountUpdaterConnectivityResponse {
            enabled: false,
            connector_config_built: None,
            detail: "Account Updater is not enabled in global config \
                (account_updater_enabled is false or credential source is none)"
                .to_string(),
        },
        Ok(Some(config)) => match connector_config::build_account_updater_connector_config(&config)
        {
            Ok(_) => AccountUpdaterConnectivityResponse {
                enabled: true,
                connector_config_built: Some(true),
                detail: "Account Updater is enabled and its connector config resolved".to_string(),
            },
            Err(error) => AccountUpdaterConnectivityResponse {
                enabled: true,
                connector_config_built: Some(false),
                detail: format!("Failed to build the Account Updater connector config: {error:?}"),
            },
        },
    };

    Ok(ApplicationResponse::Json(response))
}

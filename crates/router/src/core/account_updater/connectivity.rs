use super::{config::resolve_account_updater_config, connector_config};
use crate::{
    core::{configs::dimension_state, errors::RouterResponse},
    routes::SessionState,
    services::ApplicationResponse,
};

impl common_utils::events::ApiEventMetric for AccountUpdaterConnectivityResponse {}

/// Result of the Account Updater configuration check.
#[derive(Debug, Clone, serde::Serialize)]
pub struct AccountUpdaterConnectivityResponse {
    /// Whether Account Updater is enabled and its credentials resolved for this context.
    pub enabled: bool,
    /// Whether the credentials could be rendered into the connector config sent to UCS.
    pub connector_config_built: Option<bool>,
    /// Human-readable outcome (disabled / config error / ready).
    pub detail: String,
}

/// Reports whether Account Updater would be called and whether its credentials resolve.
///
/// Hyperswitch does not call the provider directly — UCS owns the HTTP call and the encryption —
/// so this deliberately makes no network call. Reachability is only meaningful once the `Refresh`
/// RPC is wired, and is verified there.
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

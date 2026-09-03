use super::{
    client::OfferEngineClient, config::resolve_offer_engine_credential_source,
    types::OfferEngineCredentialSource,
};
use crate::{
    core::{configs::dimension_state, errors::RouterResponse},
    routes::SessionState,
    services::ApplicationResponse,
};

impl common_utils::events::ApiEventMetric for OfferEngineConnectivityResponse {}

/// Response of the Offer Engine connectivity check.
#[derive(Debug, Clone, serde::Serialize)]
pub struct OfferEngineConnectivityResponse {
    /// Whether Offer Engine is enabled (credential source resolved) for this context.
    pub enabled: bool,
    /// Whether Offer Engine was reachable over the network. `None` when disabled.
    pub reachable: Option<bool>,
    /// HTTP status code Offer Engine returned, if reachable.
    pub status_code: Option<u16>,
    /// Human-readable outcome (config error / disabled / reachable / blocked / auth-failed).
    pub detail: String,
}

pub async fn check_offer_engine_connectivity(
    state: SessionState,
) -> RouterResponse<OfferEngineConnectivityResponse> {
    let dimensions: dimension_state::DimensionsGlobal = dimension_state::Dimensions::new();
    let resolved_config = match resolve_offer_engine_credential_source(&state, &dimensions).await {
        OfferEngineCredentialSource::None => None,
        OfferEngineCredentialSource::Application => {
            Some(OfferEngineCredentialSource::resolve_application_offer_config(&state))
        }
        OfferEngineCredentialSource::Merchant => Some(Err(error_stack::report!(
            super::types::OfferEngineError::MissingMerchantConfig(
                "credential source is 'merchant' but no merchant context is available in a \
                 global connectivity check"
                    .to_string()
            )
        ))),
    };
    let response = match resolved_config {
        Some(Err(err)) => OfferEngineConnectivityResponse {
            enabled: false,
            reachable: None,
            status_code: None,
            detail: format!("Offer Engine config could not be resolved: {err:?}"),
        },
        None => OfferEngineConnectivityResponse {
            enabled: false,
            reachable: None,
            status_code: None,
            detail: "Offer Engine is not enabled in global config \
                (offer_engine.enabled is false or credential source is none)"
                .to_string(),
        },
        Some(Ok(config)) => {
            let result = OfferEngineClient::new(config, &state.conf.trace_header.header_name)
                .check_connectivity(&state)
                .await;
            OfferEngineConnectivityResponse {
                enabled: true,
                reachable: Some(result.reachable),
                status_code: result.status_code,
                detail: result.detail,
            }
        }
    };

    Ok(ApplicationResponse::Json(response))
}

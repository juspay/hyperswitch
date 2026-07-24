use super::{client::OfferEngineClient, config::resolve_offer_engine_config};
use crate::{core::errors::RouterResponse, routes::SessionState, services::ApplicationResponse};

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
    let response = match resolve_offer_engine_config(&state, None, None).await {
        Err(err) => OfferEngineConnectivityResponse {
            enabled: false,
            reachable: None,
            status_code: None,
            detail: format!("Offer Engine config could not be resolved: {err:?}"),
        },
        Ok(None) => OfferEngineConnectivityResponse {
            enabled: false,
            reachable: None,
            status_code: None,
            detail: "Offer Engine is not enabled for this context \
                (offer_engine_enabled is false or credential source is none)"
                .to_string(),
        },
        Ok(Some(config)) => {
            let result = OfferEngineClient::new(config)
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

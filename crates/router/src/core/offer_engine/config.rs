use common_utils::errors::CustomResult;

use super::types::{OfferEngineCredentialSource, OfferEngineError, ResolvedOfferEngineConfig};
use crate::{core::configs::dimension_state, routes::SessionState};

pub async fn resolve_offer_engine_config(
    state: &SessionState,
    dimensions: &dimension_state::DimensionsGlobal,
) -> CustomResult<Option<ResolvedOfferEngineConfig>, OfferEngineError> {
    let store = state.store.as_ref();
    let superposition = state.superposition_service.as_ref();

    let enabled = dimensions
        .get_offer_engine_enabled(store, superposition, None)
        .await;

    if enabled {
        let source = dimensions
            .get_offer_engine_credential_source(store, superposition, None)
            .await;

        match source {
            OfferEngineCredentialSource::None => Ok(None),
            OfferEngineCredentialSource::Application => resolve_application_config(state).map(Some),
        }
    } else {
        Ok(None)
    }
}

fn resolve_application_config(
    state: &SessionState,
) -> CustomResult<ResolvedOfferEngineConfig, OfferEngineError> {
    let app_config = state.conf.offer_engine.as_ref().ok_or_else(|| {
        error_stack::report!(OfferEngineError::MissingApplicationConfig(
            "offer_engine application config is not set".to_string()
        ))
    })?;

    Ok(ResolvedOfferEngineConfig {
        base_url: app_config.base_url.clone(),
        api_key: app_config.api_key.clone(),
        merchant_id: app_config.merchant_id.clone(),
    })
}

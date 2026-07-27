use common_utils::errors::CustomResult;
use error_stack::ResultExt;
use hyperswitch_masking::ExposeInterface;

use super::types::{OfferEngineCredentialSource, OfferEngineError, ResolvedOfferEngineConfig};
use crate::{core::configs::dimension_state::Dimensions, routes::SessionState, utils};

pub async fn resolve_offer_engine_config(
    state: &SessionState,
) -> CustomResult<Option<ResolvedOfferEngineConfig>, OfferEngineError> {
    let dimensions = Dimensions::new();
    let store = state.store.as_ref();
    let superposition = state.superposition_service.as_ref();

    let enabled = dimensions
        .get_offer_engine_enabled(store, superposition, None)
        .await;

    // When disabled, Offer Engine is never resolved regardless of the credential source.
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

/// Resolves and validates the statically configured (application-level) Offer Engine credentials.
fn resolve_application_config(
    state: &SessionState,
) -> CustomResult<ResolvedOfferEngineConfig, OfferEngineError> {
    let app_config = state.conf.offer_engine.as_ref().ok_or_else(|| {
        error_stack::report!(OfferEngineError::MissingApplicationConfig(
            "offer_engine config is not set".to_string()
        ))
    })?;

    utils::when(app_config.base_url.is_empty(), || {
        Err(error_stack::report!(
            OfferEngineError::MissingApplicationConfig("base_url is empty".to_string())
        ))
    })?;
    let base_url = url::Url::parse(&app_config.base_url).change_context(
        OfferEngineError::MissingApplicationConfig("base_url is not a valid URL".to_string()),
    )?;
    utils::when(app_config.api_key.clone().expose().is_empty(), || {
        Err(error_stack::report!(
            OfferEngineError::MissingApplicationConfig("api_key is empty".to_string())
        ))
    })?;
    utils::when(app_config.merchant_id.is_empty(), || {
        Err(error_stack::report!(
            OfferEngineError::MissingApplicationConfig("merchant_id is empty".to_string())
        ))
    })?;

    Ok(ResolvedOfferEngineConfig {
        base_url,
        api_key: app_config.api_key.clone(),
        merchant_id: app_config.merchant_id.clone(),
    })
}

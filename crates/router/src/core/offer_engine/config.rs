use common_utils::errors::CustomResult;

use super::types::{OfferEngineCredentialSource, OfferEngineError, ResolvedOfferEngineConfig};
use crate::{
    core::configs::{self, dimension_config, dimension_state},
    routes::SessionState,
};

/// Resolve the Offer Engine config for the given dimensions. Generic over the
/// dimension so callers can target at any level (connectivity uses the global
/// dimension; payments pass merchant/org/profile for superposition targeting).
pub async fn resolve_offer_engine_config<D>(
    state: &SessionState,
    dimensions: &D,
) -> CustomResult<Option<ResolvedOfferEngineConfig>, OfferEngineError>
where
    D: dimension_state::DimensionsBase,
{
    let enabled = configs::fetch_db_config_for_dimensions::<dimension_config::OfferEngineEnabled>(
        state.store.as_ref(),
        state.superposition_service.as_ref(),
        dimensions,
        None,
    )
    .await;

    if enabled {
        resolve_offer_engine_credentials(state, dimensions).await
    } else {
        Ok(None)
    }
}

/// Resolve the Offer Engine credentials for the given dimensions by credential
/// source, independent of the `enabled` toggle. The notification path uses this:
/// an applied offer's outcome must be reported even after the feature is disabled.
pub async fn resolve_offer_engine_credentials<D>(
    state: &SessionState,
    dimensions: &D,
) -> CustomResult<Option<ResolvedOfferEngineConfig>, OfferEngineError>
where
    D: dimension_state::DimensionsBase,
{
    let source = configs::fetch_db_config_for_string_enum::<
        dimension_config::OfferEngineCredentialSource,
        OfferEngineCredentialSource,
    >(
        state.store.as_ref(),
        state.superposition_service.as_ref(),
        dimensions,
        None,
    )
    .await
    .unwrap_or(OfferEngineCredentialSource::None);

    match source {
        OfferEngineCredentialSource::None => Ok(None),
        OfferEngineCredentialSource::Application => resolve_application_config(state).map(Some),
    }
}

fn resolve_application_config(
    state: &SessionState,
) -> CustomResult<ResolvedOfferEngineConfig, OfferEngineError> {
    let app_config = state
        .conf
        .offer_engine
        .as_ref()
        .ok_or_else(|| {
            error_stack::report!(OfferEngineError::MissingApplicationConfig(
                "offer_engine application config is not set".to_string()
            ))
        })?
        .get_inner();

    Ok(ResolvedOfferEngineConfig {
        base_url: app_config.base_url.clone(),
        api_key: app_config.api_key.clone(),
        merchant_id: app_config.merchant_id.clone(),
    })
}

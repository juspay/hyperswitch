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
    let store = state.store.as_ref();
    let superposition = state.superposition_service.as_ref();

    let enabled = configs::fetch_db_config_for_dimensions::<dimension_config::OfferEngineEnabled>(
        store,
        superposition,
        dimensions,
        None,
    )
    .await;

    if enabled {
        let source = configs::fetch_db_config_for_string_enum::<
            dimension_config::OfferEngineCredentialSource,
            OfferEngineCredentialSource,
        >(store, superposition, dimensions, None)
        .await
        .unwrap_or(OfferEngineCredentialSource::None);

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

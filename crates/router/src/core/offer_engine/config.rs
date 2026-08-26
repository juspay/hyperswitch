use common_utils::errors::CustomResult;
use error_stack::ResultExt;
use hyperswitch_masking::PeekInterface;

use super::types::{OfferEngineCredentialSource, OfferEngineError, ResolvedOfferEngineConfig};
#[cfg(feature = "v1")]
use crate::types::domain;
use crate::{
    core::configs::{self, dimension_config, dimension_state},
    routes::SessionState,
};

/// Resolve which Offer Engine credential source applies for the given dimensions.
///
/// Returns [`OfferEngineCredentialSource::None`] when Offer Engine is disabled or
/// no source is configured. Callers match on the result and resolve the concrete
/// config via the per-source resolvers on [`OfferEngineCredentialSource`], so a
/// caller only loads the credentials the resolved source actually needs (e.g. the
/// merchant account is read only for the `Merchant` arm). The match can be
/// extended with profile/org sources later.
pub async fn resolve_offer_engine_credential_source<D>(
    state: &SessionState,
    dimensions: &D,
) -> OfferEngineCredentialSource
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
        configs::fetch_db_config_for_string_enum::<
            dimension_config::OfferEngineCredentialSource,
            OfferEngineCredentialSource,
        >(store, superposition, dimensions, None)
        .await
        .unwrap_or(OfferEngineCredentialSource::None)
    } else {
        OfferEngineCredentialSource::None
    }
}

impl OfferEngineCredentialSource {
    /// Resolve the Offer Engine config entirely from the application configuration.
    pub fn resolve_application_offer_config(
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

    /// Resolve the Offer Engine config from the merchant account's merchant-level
    /// config, taking only the shared base URL from the application configuration.
    #[cfg(feature = "v1")]
    pub fn resolve_merchant_offer_config(
        state: &SessionState,
        merchant_account: &domain::MerchantAccount,
    ) -> CustomResult<ResolvedOfferEngineConfig, OfferEngineError> {
        let base_url = state
            .conf
            .offer_engine
            .as_ref()
            .ok_or_else(|| {
                error_stack::report!(OfferEngineError::MissingApplicationConfig(
                    "offer_engine base_url is not set in application config".to_string()
                ))
            })?
            .get_inner()
            .base_url
            .clone();

        let merchant_offer_config =
            merchant_account.get_offer_engine_config().ok_or_else(|| {
                error_stack::report!(OfferEngineError::MissingMerchantConfig(
                    "merchant Offer Engine config is not set on the merchant account".to_string()
                ))
            })?;

        let merchant_config: api_models::admin::OfferEngineMerchantConfig = serde_json::from_value(
            merchant_offer_config.peek().clone(),
        )
        .change_context(OfferEngineError::MissingMerchantConfig(
            "merchant Offer Engine config is invalid".to_string(),
        ))?;

        Ok(ResolvedOfferEngineConfig {
            base_url,
            api_key: merchant_config.api_key,
            merchant_id: merchant_config.merchant_id,
        })
    }
}

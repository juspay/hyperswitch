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

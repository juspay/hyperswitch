use common_utils::errors::CustomResult;
use error_stack::ResultExt;
use hyperswitch_masking::PeekInterface;

use super::types::{OfferEngineCredentialSource, OfferEngineError, ResolvedOfferEngineConfig};
use crate::{
    core::configs::{self, dimension_config, dimension_state},
    routes::SessionState,
    types::domain,
};

pub async fn resolve_offer_engine_credential_source<D>(
    state: &SessionState,
    dimensions: &D,
) -> OfferEngineCredentialSource
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
        configs::fetch_db_config_for_string_enum::<
            dimension_config::OfferEngineCredentialSource,
            OfferEngineCredentialSource,
        >(
            state.store.as_ref(),
            state.superposition_service.as_ref(),
            dimensions,
            None,
        )
        .await
        .unwrap_or_else(|| {
            router_env::logger::warn!(
                "Offer Engine credential source config could not be parsed; defaulting to none"
            );
            OfferEngineCredentialSource::None
        })
    } else {
        OfferEngineCredentialSource::None
    }
}

pub async fn fetch_offer_engine_credential_source_for_notify<D>(
    state: &SessionState,
    dimensions: &D,
) -> OfferEngineCredentialSource
where
    D: dimension_state::DimensionsBase,
{
    configs::fetch_db_config_for_string_enum::<
        dimension_config::OfferEngineCredentialSource,
        OfferEngineCredentialSource,
    >(
        state.store.as_ref(),
        state.superposition_service.as_ref(),
        dimensions,
        None,
    )
    .await
    .unwrap_or_else(|| {
        router_env::logger::warn!(
            "Offer Engine credential source config could not be parsed; defaulting to none"
        );
        OfferEngineCredentialSource::None
    })
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

        let api_key = app_config.api_key.clone().ok_or_else(|| {
            error_stack::report!(OfferEngineError::MissingApplicationConfig(
                "offer_engine.api_key is required when credential source is application"
                    .to_string()
            ))
        })?;

        let merchant_id = app_config.merchant_id.clone().ok_or_else(|| {
            error_stack::report!(OfferEngineError::MissingApplicationConfig(
                "offer_engine.merchant_id is required when credential source is application"
                    .to_string()
            ))
        })?;

        Ok(ResolvedOfferEngineConfig {
            base_url: app_config.base_url.clone(),
            api_key,
            merchant_id,
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

    #[cfg(feature = "v2")]
    pub fn resolve_merchant_offer_config(
        _state: &SessionState,
        _merchant_account: &domain::MerchantAccount,
    ) -> CustomResult<ResolvedOfferEngineConfig, OfferEngineError> {
        Err(error_stack::report!(
            OfferEngineError::MissingMerchantConfig(
                "merchant-level Offer Engine credentials are only supported in v1".to_string()
            )
        ))
    }
}

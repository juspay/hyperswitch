use common_utils::errors::CustomResult;
use error_stack::ResultExt;
use hyperswitch_masking::PeekInterface;

use super::types::{OfferEngineCredentialSource, OfferEngineError, ResolvedOfferEngineConfig};
use crate::{
    core::configs::{self, dimension_config, dimension_state},
    routes::SessionState,
};

pub async fn resolve_offer_engine_config<D>(
    state: &SessionState,
    dimensions: &D,
    merchant_offer_config: Option<&common_utils::pii::SecretSerdeValue>,
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
            OfferEngineCredentialSource::Merchant => {
                resolve_merchant_config(state, merchant_offer_config).map(Some)
            }
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

fn resolve_merchant_config(
    state: &SessionState,
    merchant_offer_config: Option<&common_utils::pii::SecretSerdeValue>,
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

    let merchant_config = merchant_offer_config.ok_or_else(|| {
        error_stack::report!(OfferEngineError::MissingMerchantConfig(
            "merchant Offer Engine config is not set on the merchant account".to_string()
        ))
    })?;

    let merchant_config: api_models::admin::OfferEngineMerchantConfig = serde_json::from_value(
        merchant_config.peek().clone(),
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

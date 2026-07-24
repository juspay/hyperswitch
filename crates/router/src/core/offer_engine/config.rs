use common_utils::errors::CustomResult;
use error_stack::ResultExt;
use external_services::superposition::types::ConfigContext;
use hyperswitch_masking::ExposeInterface;

use super::types::{OfferEngineCredentialSource, OfferEngineError, ResolvedOfferEngineConfig};
use crate::{consts::superposition as superposition_consts, routes::SessionState};

pub async fn resolve_offer_engine_config(
    state: &SessionState,
    context: Option<&ConfigContext>,
    targeting_key: Option<&String>,
) -> CustomResult<Option<ResolvedOfferEngineConfig>, OfferEngineError> {
    let enabled: bool = state
        .superposition_service
        .get_config_value(
            superposition_consts::OFFER_ENGINE_ENABLED,
            context,
            targeting_key,
        )
        .await
        .change_context(OfferEngineError::EnablementUnavailable)?;

    if !enabled {
        return Ok(None);
    }

    let raw_source: String = state
        .superposition_service
        .get_config_value(
            superposition_consts::OFFER_ENGINE_CREDENTIAL_SOURCE,
            context,
            targeting_key,
        )
        .await
        .change_context(OfferEngineError::CredentialSourceUnavailable)?;

    match OfferEngineCredentialSource::parse(&raw_source)? {
        None => Ok(None),
        Some(OfferEngineCredentialSource::Application) => {
            let app_config = &state.conf.offer_engine;

            if app_config.base_url.is_empty() {
                return Err(error_stack::report!(
                    OfferEngineError::MissingApplicationConfig("base_url is empty".to_string())
                ));
            }
            let base_url = url::Url::parse(&app_config.base_url).change_context(
                OfferEngineError::MissingApplicationConfig(
                    "base_url is not a valid URL".to_string(),
                ),
            )?;
            if app_config.api_key.clone().expose().is_empty() {
                return Err(error_stack::report!(
                    OfferEngineError::MissingApplicationConfig("api_key is empty".to_string())
                ));
            }
            if app_config.merchant_id.is_empty() {
                return Err(error_stack::report!(
                    OfferEngineError::MissingApplicationConfig("merchant_id is empty".to_string())
                ));
            }

            Ok(Some(ResolvedOfferEngineConfig {
                base_url,
                api_key: app_config.api_key.clone(),
                merchant_id: app_config.merchant_id.clone(),
            }))
        }
    }
}

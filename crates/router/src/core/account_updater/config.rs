use common_utils::errors::CustomResult;

use super::types::{
    AccountUpdaterCredentialSource, AccountUpdaterError, ResolvedAccountUpdaterConfig,
};
use crate::{core::configs::dimension_state, routes::SessionState};

pub async fn resolve_account_updater_config(
    state: &SessionState,
    dimensions: &dimension_state::DimensionsGlobal,
) -> CustomResult<Option<ResolvedAccountUpdaterConfig>, AccountUpdaterError> {
    let store = state.store.as_ref();
    let superposition = state.superposition_service.as_ref();

    let enabled = dimensions
        .get_account_updater_enabled(store, superposition, None)
        .await;

    if !enabled {
        return Ok(None);
    }

    let source = dimensions
        .get_account_updater_credential_source(store, superposition, None)
        .await;

    match source {
        AccountUpdaterCredentialSource::None => Ok(None),
        AccountUpdaterCredentialSource::Application => resolve_application_config(state).map(Some),
    }
}

fn resolve_application_config(
    state: &SessionState,
) -> CustomResult<ResolvedAccountUpdaterConfig, AccountUpdaterError> {
    let app_config = state.conf.account_updater.as_ref().ok_or_else(|| {
        error_stack::report!(AccountUpdaterError::MissingApplicationConfig(
            "credential source is 'application' but the account_updater section is not configured"
                .to_string()
        ))
    })?;

    Ok(ResolvedAccountUpdaterConfig {
        base_url: app_config.base_url.clone(),
        api_key: app_config.api_key.clone(),
        merchant_id: app_config.merchant_id.clone(),
        euler_encryption_public_key: app_config.euler_encryption_public_key.clone(),
        au_decryption_pvt_key: app_config.au_decryption_pvt_key.clone(),
        card_sync_key_id: app_config.card_sync_key_id.clone(),
    })
}

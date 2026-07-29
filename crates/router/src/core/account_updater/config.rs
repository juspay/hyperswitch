use common_utils::errors::CustomResult;

use super::types::{
    AccountUpdaterCredentialSource, AccountUpdaterError, ResolvedAccountUpdaterConfig,
};
use crate::{core::configs::dimension_state, routes::SessionState};

/// Resolves whether Account Updater may be called, together with the credentials to call it with.
///
/// Evaluates the master gate first and short-circuits without reading any credential, so a
/// deployment that never seeds Superposition behaves exactly like "Account Updater off".
/// `Some(_)` is returned only when a call is permitted, so callers never re-check enablement.
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

/// Reads the statically configured Account Updater credentials.
///
/// Field-level validation happens once at startup in `AccountUpdaterConfig::validate`, so a section
/// that is present here is already complete.
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

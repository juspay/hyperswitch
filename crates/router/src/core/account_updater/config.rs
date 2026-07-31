use router_env::logger;

use super::types::{
    AccountUpdaterCredentialSource, AccountUpdaterGateDecision, ResolvedAccountUpdaterConfig,
    SkipReason,
};
use crate::{configs::settings, core::configs::dimension_state, routes::SessionState};

/// Returns `Proceed` only when a call is permitted, so callers never re-check enablement.
pub async fn resolve_account_updater_config(
    state: &SessionState,
    dimensions: &dimension_state::DimensionsGlobal,
) -> AccountUpdaterGateDecision {
    let store = state.store.as_ref();
    let superposition = state.superposition_service.as_ref();

    if !dimensions
        .get_account_updater_enabled(store, superposition, None)
        .await
    {
        return AccountUpdaterGateDecision::Skipped(SkipReason::GateDisabled);
    }

    match dimensions
        .get_account_updater_credential_source(store, superposition, None)
        .await
    {
        AccountUpdaterCredentialSource::None => {
            AccountUpdaterGateDecision::Skipped(SkipReason::CredentialSourceNone)
        }
        AccountUpdaterCredentialSource::Application => match resolve_application_config(state) {
            Some(config) => AccountUpdaterGateDecision::Proceed(config),
            None => {
                logger::warn!(
                    "Account Updater credential source is 'application' but the account_updater \
                     section is not configured"
                );
                AccountUpdaterGateDecision::Skipped(SkipReason::CredentialsUnavailable)
            }
        },
    }
}

fn resolve_application_config(state: &SessionState) -> Option<ResolvedAccountUpdaterConfig> {
    let settings::AccountUpdaterConfig::Juspay(juspay) =
        state.conf.account_updater.as_ref()?.get_inner();

    Some(ResolvedAccountUpdaterConfig {
        base_url: juspay.base_url.clone(),
        api_key: juspay.api_key.clone(),
        merchant_id: juspay.merchant_id.clone(),
        euler_encryption_public_key: juspay.euler_encryption_public_key.clone(),
        au_decryption_pvt_key: juspay.au_decryption_pvt_key.clone(),
        card_sync_key_id: juspay.card_sync_key_id.clone(),
        refresh_timeout_ms: juspay.refresh_timeout_ms,
    })
}

use router_env::logger;

use super::types::{
    AccountUpdaterCredentialSource, JuspayCredentials, ResolvedAccountUpdaterConfig, SkipReason,
};
use crate::{configs::settings, core::configs::dimension_state, routes::SessionState};

pub async fn resolve_account_updater_config(
    state: &SessionState,
    dimensions: &dimension_state::DimensionsGlobal,
) -> Result<ResolvedAccountUpdaterConfig, SkipReason> {
    let store = state.store.as_ref();
    let superposition = state.superposition_service.as_ref();

    if !dimensions
        .get_account_updater_enabled(store, superposition, None)
        .await
    {
        return Err(SkipReason::GateDisabled);
    }

    match dimensions
        .get_account_updater_credential_source(store, superposition, None)
        .await
    {
        AccountUpdaterCredentialSource::None => None,
        AccountUpdaterCredentialSource::Application => resolve_application_config(state),
    }
    .ok_or(SkipReason::CredentialSourceNone)
}

fn resolve_application_config(state: &SessionState) -> Option<ResolvedAccountUpdaterConfig> {
    let Some(account_updater) = state.conf.account_updater.as_ref() else {
        logger::warn!(
            "Account Updater credential source is 'application' but the account_updater \
             section is not configured"
        );
        return None;
    };

    match account_updater.get_inner() {
        settings::AccountUpdaterConfig::Juspay(juspay) => {
            Some(ResolvedAccountUpdaterConfig::Juspay(JuspayCredentials {
                base_url: juspay.base_url.clone(),
                api_key: juspay.api_key.clone(),
                merchant_id: juspay.merchant_id.clone(),
                euler_encryption_public_key: juspay.euler_encryption_public_key.clone(),
                au_decryption_pvt_key: juspay.au_decryption_pvt_key.clone(),
                card_sync_key_id: juspay.card_sync_key_id.clone(),
            }))
        }
    }
}

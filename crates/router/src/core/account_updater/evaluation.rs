use common_enums::StorageType;
use router_env::{instrument, tracing};

use super::{
    config::resolve_account_updater_config,
    eligibility::evaluate_eligibility,
    raw_card::fetch_card_for_sync,
    refresh::refresh_card,
    types::{AccountUpdaterGateDecision, AccountUpdaterTerminalState},
};
use crate::{core::configs::dimension_state, routes::SessionState, types::domain};

/// Always yields a terminal state, so nothing here can fail the surrounding request.
#[instrument(skip_all)]
pub async fn evaluate(
    state: &SessionState,
    platform: &domain::Platform,
    profile: &domain::Profile,
    payment_method: &domain::PaymentMethod,
    storage_type: StorageType,
    dimensions: &dimension_state::DimensionsGlobal,
) -> AccountUpdaterTerminalState {
    let config = match resolve_account_updater_config(state, dimensions).await {
        AccountUpdaterGateDecision::Proceed(config) => config,
        AccountUpdaterGateDecision::Skipped(reason) => {
            return AccountUpdaterTerminalState::Skipped(reason)
        }
    };

    let eligible_card = match evaluate_eligibility(payment_method) {
        Ok(eligible_card) => eligible_card,
        Err(reason) => return AccountUpdaterTerminalState::Skipped(reason),
    };

    let sync_card = match fetch_card_for_sync(
        state,
        platform,
        profile,
        payment_method,
        storage_type,
        &eligible_card,
    )
    .await
    {
        Ok(sync_card) => sync_card,
        Err(failure) => return AccountUpdaterTerminalState::Failed(failure),
    };

    match refresh_card(state, platform, profile, &config, sync_card).await {
        Ok(outcome) => AccountUpdaterTerminalState::Refreshed(outcome),
        Err(failure) => AccountUpdaterTerminalState::Failed(failure),
    }
}

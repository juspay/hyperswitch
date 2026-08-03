use common_enums::StorageType;
use router_env::{instrument, logger, metric_attributes, tracing};

use super::{
    config::resolve_account_updater_config,
    eligibility::evaluate_eligibility,
    raw_card::fetch_card_for_sync,
    refresh::refresh_card,
    types::{AccountUpdaterTerminalState, RefreshOutcome},
};
use crate::{
    core::{configs::dimension_state, metrics},
    routes::SessionState,
    types::domain,
};

/// Records a terminal state for every path, so nothing here can fail the surrounding request.
#[instrument(skip_all)]
pub async fn evaluate(
    state: &SessionState,
    platform: &domain::Platform,
    profile: &domain::Profile,
    payment_method: &domain::PaymentMethod,
    storage_type: StorageType,
    dimensions: &dimension_state::DimensionsGlobal,
) {
    let terminal_state = run(
        state,
        platform,
        profile,
        payment_method,
        storage_type,
        dimensions,
    )
    .await
    .map_or_else(
        |terminal_state| terminal_state,
        AccountUpdaterTerminalState::Refreshed,
    );

    let (state_label, detail) = terminal_state.as_labels();

    metrics::ACCOUNT_UPDATER_EVALUATION_COUNT.add(
        1,
        metric_attributes!(("state", state_label), ("detail", detail)),
    );

    logger::info!(
        account_updater_state = state_label,
        account_updater_detail = detail,
        "Account Updater evaluation completed"
    );
}

async fn run(
    state: &SessionState,
    platform: &domain::Platform,
    profile: &domain::Profile,
    payment_method: &domain::PaymentMethod,
    storage_type: StorageType,
    dimensions: &dimension_state::DimensionsGlobal,
) -> Result<RefreshOutcome, AccountUpdaterTerminalState> {
    let config = resolve_account_updater_config(state, dimensions).await?;

    let eligible_card = evaluate_eligibility(payment_method)?;

    let sync_card = fetch_card_for_sync(
        state,
        platform,
        profile,
        payment_method,
        storage_type,
        &eligible_card,
    )
    .await?;

    refresh_card(state, platform, profile, &config, sync_card)
        .await
        .map_err(Into::into)
}

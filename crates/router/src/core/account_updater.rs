mod config;
mod eligibility;
mod refresh;
pub mod types;

use common_enums::StorageType;
use common_utils::errors::CustomResult;
use router_env::{instrument, logger, tracing};
use unified_connector_service_client::payments as payments_grpc;

use self::{
    config::resolve_account_updater_config,
    eligibility::check_eligibility_and_fetch_payment_method,
    refresh::request_account_updater_refresh,
    types::{AccountUpdaterError, ResolvedAccountUpdaterConfig},
};
use crate::{
    core::configs::dimension_state, events::account_updater as account_updater_events,
    routes::SessionState, types::domain,
};

#[instrument(skip_all)]
pub async fn run_account_updater<D>(
    state: &SessionState,
    platform: &domain::Platform,
    profile: &domain::Profile,
    payment_method: &domain::PaymentMethod,
    storage_type: StorageType,
    dimensions: &D,
) where
    D: dimension_state::DimensionsBase,
{
    let config = match resolve_account_updater_config(state, dimensions).await {
        Ok(Some(config)) => config,
        Ok(None) => {
            logger::debug!("Account Updater is not enabled for these dimensions");
            return;
        }
        Err(error) => {
            logger::warn!(?error, "Account Updater config could not be resolved");
            return;
        }
    };

    let started_at = std::time::Instant::now();

    let outcome = refresh_stored_payment_method(
        state,
        platform,
        profile,
        payment_method,
        storage_type,
        &config,
    )
    .await;

    match &outcome {
        Ok(refresh_outcome) => logger::info!(
            account_updater_outcome = refresh_outcome.as_str_name(),
            "Account Updater refresh completed"
        ),
        Err(error) => logger::info!(?error, "Account Updater refresh did not complete"),
    }

    // In shadow mode this event is the only record of the evaluation.
    if state.conf.events.emit_account_updater_events {
        let event = account_updater_events::KafkaAccountUpdaterEvent::new(
            state.request_id.as_ref().map(|id| id.to_string()),
            platform.get_processor().get_account().get_id(),
            profile.get_id(),
            payment_method,
            &outcome,
            started_at.elapsed().as_millis(),
        );

        state.event_handler.log_event(&event);
    }
}

async fn refresh_stored_payment_method(
    state: &SessionState,
    platform: &domain::Platform,
    profile: &domain::Profile,
    payment_method: &domain::PaymentMethod,
    storage_type: StorageType,
    config: &ResolvedAccountUpdaterConfig,
) -> CustomResult<payments_grpc::CardRefreshOutcome, AccountUpdaterError> {
    let refreshable_payment_method = check_eligibility_and_fetch_payment_method(
        state,
        platform,
        profile,
        payment_method,
        storage_type,
        config,
    )
    .await?;

    request_account_updater_refresh(state, platform, profile, config, refreshable_payment_method)
        .await
}

mod config;
mod eligibility;
mod refresh;
mod store;
pub mod types;

use api_models::payment_methods::RawPaymentMethodData;
use common_utils::errors::CustomResult;
use error_stack::report;
use router_env::{instrument, logger, tracing};

pub use self::store::apply_card_refresh_result;
use self::{
    config::resolve_account_updater_config,
    eligibility::{check_eligibility_and_build_payment_method, check_vault_eligibility},
    refresh::request_account_updater_refresh,
    types::{AccountUpdaterError, RefreshResult, ResolvedAccountUpdaterConfig},
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
    raw_payment_method_data: Option<&RawPaymentMethodData>,
    dimensions: &D,
) -> CustomResult<RefreshResult, AccountUpdaterError>
where
    D: dimension_state::DimensionsBase,
{
    let config = match resolve_account_updater_config(state, dimensions).await {
        Ok(Some(config)) => config,
        Ok(None) => {
            logger::debug!("Account Updater is not enabled for these dimensions");
            Err(report!(AccountUpdaterError::NotEnabled))?
        }
        Err(error) => {
            logger::warn!(?error, "Account Updater config could not be resolved");
            Err(error)?
        }
    };

    let started_at = std::time::Instant::now();

    let refresh_result = refresh_stored_payment_method(
        state,
        platform,
        profile,
        payment_method,
        raw_payment_method_data,
        &config,
    )
    .await;

    if let Err(error) = &refresh_result {
        logger::warn!(?error, "Account Updater refresh did not complete");
    }

    let event = account_updater_events::KafkaAccountUpdaterEvent::new(
        state.request_id.as_ref().map(|id| id.to_string()),
        platform.get_processor().get_account().get_id(),
        profile.get_id(),
        payment_method,
        &refresh_result,
        started_at.elapsed().as_millis(),
    );

    state.event_handler.log_event(&event);

    refresh_result
}

async fn refresh_stored_payment_method(
    state: &SessionState,
    platform: &domain::Platform,
    profile: &domain::Profile,
    payment_method: &domain::PaymentMethod,
    raw_payment_method_data: Option<&RawPaymentMethodData>,
    config: &ResolvedAccountUpdaterConfig,
) -> CustomResult<RefreshResult, AccountUpdaterError> {
    check_vault_eligibility(state, platform, profile).await?;

    let refreshable_payment_method = check_eligibility_and_build_payment_method(
        payment_method,
        raw_payment_method_data,
        config,
    )?;

    request_account_updater_refresh(state, platform, profile, config, refreshable_payment_method)
        .await
}

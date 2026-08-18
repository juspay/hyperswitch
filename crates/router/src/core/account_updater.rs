mod config;
mod eligibility;
mod refresh;
mod store;
pub mod types;

use api_models::payment_methods::RawPaymentMethodData;
use common_utils::errors::CustomResult;
use router_env::{instrument, logger, tracing};

pub use self::store::create_payment_method_for_updated_card;
use self::{
    config::resolve_account_updater_config,
    eligibility::{
        check_deployment_stores_fingerprints, check_eligibility_and_build_payment_method,
    },
    refresh::request_account_updater_refresh,
    types::{AccountUpdaterError, RefreshResult, ResolvedAccountUpdaterConfig},
};
use crate::{core::configs::dimension_state, routes::SessionState, types::domain};

#[instrument(skip_all)]
pub async fn run_account_updater<D>(
    state: &SessionState,
    platform: &domain::Platform,
    profile: &domain::Profile,
    payment_method: &domain::PaymentMethod,
    raw_payment_method_data: Option<&RawPaymentMethodData>,
    dimensions: &D,
) -> Option<RefreshResult>
where
    D: dimension_state::DimensionsBase,
{
    let config = match resolve_account_updater_config(state, dimensions).await {
        Ok(Some(config)) => config,
        Ok(None) => {
            logger::debug!("Account Updater is not enabled for these dimensions");
            return None;
        }
        Err(error) => {
            logger::warn!(?error, "Account Updater config could not be resolved");
            return None;
        }
    };

    refresh_stored_payment_method(
        state,
        platform,
        profile,
        payment_method,
        raw_payment_method_data,
        &config,
    )
    .await
    .inspect(|refresh_result| match refresh_result {
        RefreshResult::Card(card_result) => logger::info!(
            account_updater_outcome = card_result.outcome.as_str_name(),
            "Account Updater refresh completed"
        ),
    })
    .inspect_err(|error| logger::warn!(?error, "Account Updater refresh did not complete"))
    .ok()
}

async fn refresh_stored_payment_method(
    state: &SessionState,
    platform: &domain::Platform,
    profile: &domain::Profile,
    payment_method: &domain::PaymentMethod,
    raw_payment_method_data: Option<&RawPaymentMethodData>,
    config: &ResolvedAccountUpdaterConfig,
) -> CustomResult<RefreshResult, AccountUpdaterError> {
    check_deployment_stores_fingerprints(state, platform, profile).await?;

    let refreshable_payment_method = check_eligibility_and_build_payment_method(
        payment_method,
        raw_payment_method_data,
        config,
    )?;

    request_account_updater_refresh(state, platform, profile, config, refreshable_payment_method)
        .await
}

use api_models::payment_methods::RawPaymentMethodData;
use common_enums::StorageType;
use router_env::{instrument, logger, tracing};

use super::types::{AccountUpdaterFailure, EligibleCard, SyncCard};
use crate::{
    core::payment_methods::RawPaymentMethodFetchAccess, routes::SessionState, types::domain,
};

/// Unvaults on an internal grant, independent of whatever the caller was granted.
#[instrument(skip_all)]
pub async fn fetch_card_for_sync(
    state: &SessionState,
    platform: &domain::Platform,
    profile: &domain::Profile,
    payment_method: &domain::PaymentMethod,
    storage_type: StorageType,
    eligible_card: &EligibleCard,
) -> Result<SyncCard, AccountUpdaterFailure> {
    let raw_payment_method_data = RawPaymentMethodFetchAccess::Allowed
        .get_raw_payment_method_data(state, platform, profile, payment_method, storage_type)
        .await
        .map_err(|error| {
            logger::warn!(?error, "Account Updater could not unvault the stored card");
            AccountUpdaterFailure::RawCardUnavailable
        })?;

    let card_details = match raw_payment_method_data {
        Some(RawPaymentMethodData::Card(card_details)) => card_details,
        Some(RawPaymentMethodData::CardWithNT(details)) => details.card_details,
        _ => return Err(AccountUpdaterFailure::RawCardUnavailable),
    };

    Ok(SyncCard {
        card_number: card_details.card_number,
        expiry_month: card_details.card_exp_month,
        expiry_year: card_details.card_exp_year,
        // From stored metadata, never the unvaulted card.
        network: eligible_card.network.clone(),
    })
}

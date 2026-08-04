use std::str::FromStr;

use api_models::payment_methods::RawPaymentMethodData;
use common_enums::StorageType;
use router_env::{instrument, logger, tracing};
use unified_connector_service_cards::CardNumber;

use super::types::{EligibleCard, SkipReason, SyncCard};
use crate::{
    core::payment_methods::RawPaymentMethodFetchAccess, routes::SessionState, types::domain,
};

#[instrument(skip_all)]
pub async fn fetch_card_for_sync(
    state: &SessionState,
    platform: &domain::Platform,
    profile: &domain::Profile,
    payment_method: &domain::PaymentMethod,
    storage_type: StorageType,
    eligible_card: &EligibleCard,
) -> Result<SyncCard, SkipReason> {
    let raw_payment_method_data = RawPaymentMethodFetchAccess::Allowed
        .get_raw_payment_method_data(state, platform, profile, payment_method, storage_type)
        .await
        .map_err(|error| {
            logger::warn!(?error, "Account Updater could not unvault the stored card");
            SkipReason::RawCardUnavailable
        })?;

    let card_details = match raw_payment_method_data {
        Some(RawPaymentMethodData::Card(card_details)) => card_details,
        Some(RawPaymentMethodData::CardWithNT(details)) => details.card_details,
        Some(RawPaymentMethodData::BankDebit(_) | RawPaymentMethodData::ProxyCard(_)) | None => {
            return Err(SkipReason::RawCardUnavailable)
        }
    };

    let card_number =
        CardNumber::from_str(&card_details.card_number.get_card_no()).map_err(|_| {
            logger::warn!("Account Updater unvaulted a card number that UCS rejected as invalid");
            SkipReason::RawCardUnavailable
        })?;

    Ok(SyncCard {
        card_number,
        expiry_month: card_details.card_exp_month,
        expiry_year: card_details.card_exp_year,
        // From stored metadata, never the unvaulted card.
        network: eligible_card.network.clone(),
    })
}

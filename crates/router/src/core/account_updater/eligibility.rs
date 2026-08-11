use std::str::FromStr;

use api_models::payment_methods::RawPaymentMethodData;
use common_enums::{CardNetwork, PaymentMethod, PaymentMethodStatus, StorageType};
use common_utils::errors::CustomResult;
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::payment_method_data::PaymentMethodsData;
use router_env::{instrument, tracing};
use unified_connector_service_cards::CardNumber;
use unified_connector_service_client::payments as payments_grpc;

use super::types::AccountUpdaterError;
use crate::{
    core::payment_methods::RawPaymentMethodFetchAccess,
    routes::SessionState,
    types::{domain, transformers::ForeignFrom},
};

#[instrument(skip_all)]
pub async fn check_eligibility_and_fetch_payment_method(
    state: &SessionState,
    platform: &domain::Platform,
    profile: &domain::Profile,
    payment_method: &domain::PaymentMethod,
    storage_type: StorageType,
) -> CustomResult<payments_grpc::PaymentMethod, AccountUpdaterError> {
    if payment_method.status != PaymentMethodStatus::Active {
        return Err(report!(AccountUpdaterError::PaymentMethodNotActive));
    }

    match payment_method.get_payment_method_type() {
        Some(PaymentMethod::Card) => {
            check_stored_card_eligibility_and_fetch_details(
                state,
                platform,
                profile,
                payment_method,
                storage_type,
            )
            .await
        }
        _ => Err(report!(AccountUpdaterError::PaymentMethodNotACard)),
    }
}

async fn check_stored_card_eligibility_and_fetch_details(
    state: &SessionState,
    platform: &domain::Platform,
    profile: &domain::Profile,
    payment_method: &domain::PaymentMethod,
    storage_type: StorageType,
) -> CustomResult<payments_grpc::PaymentMethod, AccountUpdaterError> {
    let card_details = match payment_method
        .payment_method_data
        .as_ref()
        .map(|payment_method_data| payment_method_data.get_inner())
    {
        Some(PaymentMethodsData::Card(card_details)) => card_details,
        _ => {
            return Err(report!(AccountUpdaterError::PaymentMethodNotACard)
                .attach_printable("Stored payment method data holds no card details"))
        }
    };

    if !matches!(
        card_details.card_network,
        Some(CardNetwork::Visa | CardNetwork::Mastercard)
    ) {
        return Err(report!(AccountUpdaterError::UnsupportedNetwork));
    }

    let raw_payment_method_data = RawPaymentMethodFetchAccess::Allowed
        .get_raw_payment_method_data(state, platform, profile, payment_method, storage_type)
        .await
        .change_context(AccountUpdaterError::CardUnusable)
        .attach_printable("Account Updater could not unvault the stored card")?;

    build_refreshable_payment_method(raw_payment_method_data)
}

fn build_refreshable_payment_method(
    raw_payment_method_data: Option<RawPaymentMethodData>,
) -> CustomResult<payments_grpc::PaymentMethod, AccountUpdaterError> {
    let card_details = match raw_payment_method_data {
        Some(RawPaymentMethodData::Card(card_details)) => card_details,
        Some(RawPaymentMethodData::CardWithNT(details)) => details.card_details,
        Some(RawPaymentMethodData::BankDebit(_) | RawPaymentMethodData::ProxyCard(_)) | None => {
            return Err(report!(AccountUpdaterError::CardUnusable)
                .attach_printable("Unvaulted payment method data holds no card"))
        }
    };

    let card_number = CardNumber::from_str(&card_details.card_number.get_card_no())
        .map_err(|_| report!(AccountUpdaterError::CardUnusable))
        .attach_printable("Failed to parse the unvaulted card number")?;

    let network = card_details
        .card_network
        .ok_or_else(|| report!(AccountUpdaterError::CardUnusable))
        .attach_printable("Unvaulted card carries no network")?;

    Ok(payments_grpc::PaymentMethod {
        payment_method: Some(payments_grpc::payment_method::PaymentMethod::CardWithNoCvc(
            payments_grpc::CardDetailsWithNoCvc {
                card_number: Some(card_number),
                card_exp_month: Some(card_details.card_exp_month),
                card_exp_year: Some(card_details.card_exp_year),
                card_network: Some(i32::from(payments_grpc::CardNetwork::foreign_from(network))),
                card_holder_name: None,
                card_issuer: None,
                card_type: None,
                card_issuing_country_alpha2: None,
                bank_code: None,
                nick_name: None,
            },
        )),
    })
}

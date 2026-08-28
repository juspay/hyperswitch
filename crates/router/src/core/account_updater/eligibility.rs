use std::str::FromStr;

use api_models::payment_methods::{CardDetail, RawPaymentMethodData};
use common_enums::{PaymentMethod, PaymentMethodStatus};
use common_utils::{errors::CustomResult, fp_utils::when};
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::payment_method_data::PaymentMethodsData;
use router_env::{instrument, tracing};
use unified_connector_service_cards::CardNumber;
use unified_connector_service_client::payments as payments_grpc;

use super::types::{AccountUpdaterError, ResolvedAccountUpdaterConfig};
use crate::types::{domain, transformers::ForeignFrom};

#[instrument(skip_all)]
pub fn check_eligibility_and_build_payment_method(
    payment_method: &domain::PaymentMethod,
    raw_payment_method_data: Option<&RawPaymentMethodData>,
    config: &ResolvedAccountUpdaterConfig,
) -> CustomResult<payments_grpc::PaymentMethod, AccountUpdaterError> {
    when(payment_method.status != PaymentMethodStatus::Active, || {
        Err(report!(AccountUpdaterError::PaymentMethodNotActive))
    })?;

    match payment_method.get_payment_method_type() {
        Some(PaymentMethod::Card) => check_stored_card_eligibility_and_build_payment_method(
            payment_method,
            raw_payment_method_data,
            config,
        ),
        _ => Err(report!(AccountUpdaterError::PaymentMethodNotACard)),
    }
}

fn check_stored_card_eligibility_and_build_payment_method(
    payment_method: &domain::PaymentMethod,
    raw_payment_method_data: Option<&RawPaymentMethodData>,
    config: &ResolvedAccountUpdaterConfig,
) -> CustomResult<payments_grpc::PaymentMethod, AccountUpdaterError> {
    let card_details = payment_method
        .payment_method_data
        .as_ref()
        .map(|payment_method_data| payment_method_data.get_inner())
        .and_then(|payment_method_data| match payment_method_data {
            PaymentMethodsData::Card(card_details) => Some(card_details),
            _ => None,
        })
        .ok_or_else(|| report!(AccountUpdaterError::PaymentMethodNotACard))
        .attach_printable("Stored payment method data holds no card details")?;

    card_details
        .card_network
        .as_ref()
        .filter(|card_network| config.supported_card_networks().contains(*card_network))
        .ok_or_else(|| report!(AccountUpdaterError::UnsupportedNetwork))
        .attach_printable("Stored card network is not configured for Account Updater")?;

    build_refreshable_payment_method(raw_payment_method_data)
}

fn build_refreshable_payment_method(
    raw_payment_method_data: Option<&RawPaymentMethodData>,
) -> CustomResult<payments_grpc::PaymentMethod, AccountUpdaterError> {
    let card_details: &CardDetail = match raw_payment_method_data {
        Some(RawPaymentMethodData::Card(card_details)) => Some(card_details),
        Some(RawPaymentMethodData::CardWithNT(details)) => Some(&details.card_details),
        Some(RawPaymentMethodData::BankDebit(_) | RawPaymentMethodData::ProxyCard(_)) | None => {
            None
        }
    }
    .ok_or_else(|| report!(AccountUpdaterError::CardUnusable))
    .attach_printable("Unvaulted payment method data holds no card")?;

    let card_number = CardNumber::from_str(&card_details.card_number.get_card_no())
        .change_context(AccountUpdaterError::CardUnusable)
        .attach_printable("Failed to parse the unvaulted card number")?;

    let network = card_details
        .card_network
        .clone()
        .ok_or_else(|| report!(AccountUpdaterError::CardUnusable))
        .attach_printable("Unvaulted card carries no network")?;

    Ok(payments_grpc::PaymentMethod {
        payment_method: Some(payments_grpc::payment_method::PaymentMethod::CardWithNoCvc(
            payments_grpc::CardDetailsWithNoCvc {
                card_number: Some(card_number),
                card_exp_month: Some(card_details.card_exp_month.clone()),
                card_exp_year: Some(card_details.card_exp_year.clone()),
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

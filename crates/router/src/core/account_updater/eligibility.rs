use common_enums::{CardNetwork, PaymentMethod, PaymentMethodStatus};
use hyperswitch_domain_models::payment_method_data::PaymentMethodsData;

use super::types::{EligibleCard, SkipReason};
use crate::types::domain;

/// Stored-metadata checks only, so an ineligible card is never unvaulted.
pub fn evaluate_eligibility(
    payment_method: &domain::PaymentMethod,
) -> Result<EligibleCard, SkipReason> {
    evaluate_stored_card(
        payment_method.get_payment_method_type(),
        payment_method.status,
        payment_method
            .payment_method_data
            .clone()
            .map(|payment_method_data| payment_method_data.into_inner())
            .as_ref(),
    )
}

fn evaluate_stored_card(
    payment_method_type: Option<PaymentMethod>,
    status: PaymentMethodStatus,
    payment_method_data: Option<&PaymentMethodsData>,
) -> Result<EligibleCard, SkipReason> {
    if payment_method_type != Some(PaymentMethod::Card) {
        return Err(SkipReason::NotACard);
    }

    if status != PaymentMethodStatus::Active {
        return Err(SkipReason::PaymentMethodNotActive);
    }

    let card_details = match payment_method_data {
        Some(PaymentMethodsData::Card(card_details)) => card_details,
        Some(_) => return Err(SkipReason::NotACard),
        None => return Err(SkipReason::NetworkUnknown),
    };

    match &card_details.card_network {
        Some(network @ (CardNetwork::Visa | CardNetwork::Mastercard)) => Ok(EligibleCard {
            network: network.clone(),
        }),
        Some(_) => Err(SkipReason::UnsupportedNetwork),
        None => Err(SkipReason::NetworkUnknown),
    }
}

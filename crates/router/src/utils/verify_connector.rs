use api_models::enums::Connector;
use error_stack::ResultExt;

use crate::{core::errors, types::api};

pub fn generate_card_from_details(
    card_number: String,
    card_exp_year: String,
    card_exp_month: String,
    card_cvv: String,
) -> errors::RouterResult<api::Card> {
    Ok(api::Card {
        card_number: card_number
            .parse()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error while parsing card number")?,
        card_issuer: None,
        card_cvc: masking::Secret::new(card_cvv),
        card_network: None,
        card_exp_year: masking::Secret::new(card_exp_year),
        card_exp_month: masking::Secret::new(card_exp_month),
        card_holder_name: Some(masking::Secret::new("HyperSwitch".to_string())),
        nick_name: None,
        card_type: None,
        card_issuing_country: None,
        bank_code: None,
    })
}

pub fn get_test_card_details(connector_name: Connector) -> errors::RouterResult<Option<api::Card>> {
    match connector_name {
        Connector::Stripe => Some(generate_card_from_details(
            "4242424242424242".to_string(),
            "2025".to_string(),
            "12".to_string(),
            "100".to_string(),
        ))
        .transpose(),
        Connector::Paypal => Some(generate_card_from_details(
            "4111111111111111".to_string(),
            "2025".to_string(),
            "02".to_string(),
            "123".to_string(),
        ))
        .transpose(),
        _ => Ok(None),
    }
}

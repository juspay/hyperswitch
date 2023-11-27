use crate::{core::errors, types::api};
use api_models::enums::Connector;

pub fn generate_card_from_details(
    card_number: String,
    card_exp_year: String,
    card_exp_month: String,
    card_cvv: String,
) -> Result<api::Card, errors::ApiErrorResponse> {
    Ok(api::Card {
        card_number: card_number
            .parse()
            .map_err(|_| errors::ApiErrorResponse::InternalServerError)?,
        card_issuer: None,
        card_cvc: masking::Secret::new(card_cvv),
        card_network: None,
        card_exp_year: masking::Secret::new(card_exp_year),
        card_exp_month: masking::Secret::new(card_exp_month),
        card_holder_name: masking::Secret::new("HyperSwitch".to_string()),
        nick_name: None,
        card_type: None,
        card_issuing_country: None,
        bank_code: None,
    })
}

pub fn get_test_card_details(
    connector_name: Connector,
) -> Result<api::Card, errors::ApiErrorResponse> {
    match connector_name {
        Connector::Stripe => generate_card_from_details(
            "4242424242424242".to_string(),
            "2025".to_string(),
            "12".to_string(),
            "100".to_string(),
        ),
        Connector::Paypal => generate_card_from_details(
            "4111111111111111".to_string(),
            "2025".to_string(),
            "02".to_string(),
            "123".to_string(),
        ),
        _ => Err(errors::ApiErrorResponse::IncorrectConnectorNameGiven),
    }
}

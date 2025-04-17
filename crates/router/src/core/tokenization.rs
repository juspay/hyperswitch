#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
use std::sync::Arc;

#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
use error_stack::{IntoReport, ResultExt};
#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
use masking::Secret;
#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
use router_env::{instrument, tracing};
#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
use serde_json::Value;

#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
use crate::{
    core::errors::{self, RouterResult},
    routes::AppState,
    services,
    types::{
        api,
        domain::MerchantAccount,
    },
};

#[instrument(skip_all)]
#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
pub async fn tokenize_card(
    state: Arc<AppState>,
    merchant_account: MerchantAccount,
    req: Value,
) -> RouterResult<api::TokenizationResponse> {
    // Extract card details from JSON request
    let card_number = req["card_number"]
        .as_str()
        .ok_or(errors::ApiErrorResponse::InvalidCardNumber)?;
    let expiry_month = req["expiry_month"]
        .as_str()
        .ok_or(errors::ApiErrorResponse::InvalidExpiryDate)?;
    let expiry_year = req["expiry_year"]
        .as_str()
        .ok_or(errors::ApiErrorResponse::InvalidExpiryDate)?;
    let name_on_card = req["name_on_card"].as_str();
    let card_cvc = req["card_cvc"].as_str();

    // Create a tokenization request
    let tokenization_req = api::TokenizationRequest {
        card_number: Secret::new(card_number.to_string()),
        expiry_month: Secret::new(expiry_month.to_string()),
        expiry_year: Secret::new(expiry_year.to_string()),
        name_on_card: name_on_card.map(|s| Secret::new(s.to_string())),
        card_cvc: card_cvc.map(|s| Secret::new(s.to_string())),
    };

    // Call the tokenization service
    let tokenization_response = services::tokenization::tokenize_card(
        state,
        merchant_account,
        tokenization_req,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to tokenize card")?;

    Ok(tokenization_response)
}

#[instrument(skip_all)]
#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
pub async fn detokenize_card(
    state: Arc<AppState>,
    merchant_account: MerchantAccount,
    token: String,
) -> RouterResult<api::DetokenizationResponse> {
    // Call the detokenization service
    let detokenization_response = services::tokenization::detokenize_card(
        state,
        merchant_account,
        token,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to detokenize card")?;

    Ok(detokenization_response)
}

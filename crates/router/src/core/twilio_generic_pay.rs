use api_models::twilio_generic_pay::{
    TwilioGenericPayRequest, TwilioGenericPayResponse, TwilioPayMethod,
};
use common_utils::types::{AmountConvertor, StringMajorUnitForConnector};

use crate::core::generic_pay::{self, Context, Failure, GenericPayAdapter, Parameters};

/// What Twilio can ask for that this endpoint does not do; the rest are shared, in
/// [`generic_pay::error_codes`].
mod error_codes {
    pub const TOKENIZE_UNSUPPORTED: &str = "TOKENIZE_UNSUPPORTED";
    pub const BANK_ACCOUNT_UNSUPPORTED: &str = "BANK_ACCOUNT_UNSUPPORTED";
}

struct TwilioCardDetails {
    card_number: cards::CardNumber,
    card_exp_month: hyperswitch_masking::Secret<String>,
    card_exp_year: hyperswitch_masking::Secret<String>,
    card_cvc: hyperswitch_masking::Secret<String>,
}

/// Twilio's Generic Pay Connector. Everything but the request and response shapes is shared with
/// the other providers, in [`generic_pay`].
pub struct TwilioGenericPay;

#[async_trait::async_trait]
impl GenericPayAdapter for TwilioGenericPay {
    type Request = TwilioGenericPayRequest;
    type Response = TwilioGenericPayResponse;

    fn transaction_reference(request: &Self::Request) -> String {
        request.transaction_id.clone()
    }

    fn parameters(request: &Self::Request) -> Option<&Parameters> {
        request.parameters.as_ref()
    }

    /// Maps the `<Pay>` verb's fields onto a card payment.
    fn build_payments_request(
        request: &Self::Request,
        context: Context,
    ) -> Result<api_models::payments::PaymentsRequest, Failure> {
        if request.method != TwilioPayMethod::Charge {
            return Err(Failure::new(
                error_codes::TOKENIZE_UNSUPPORTED,
                "Tokenization is not supported on this endpoint",
            ));
        }

        if request.bankaccountnumber.is_some() || request.routingnumber.is_some() {
            return Err(Failure::new(
                error_codes::BANK_ACCOUNT_UNSUPPORTED,
                "Bank account payments are not supported on this endpoint",
            ));
        }

        let card = extract_card(request)?;

        let currency = request
            .currency_code
            .ok_or_else(|| invalid_request("currency_code is required"))?;

        let amount = request
            .amount
            .clone()
            .ok_or_else(|| invalid_request("amount is required"))?;

        let amount = StringMajorUnitForConnector
            .convert_back(amount, currency)
            .map_err(|error| invalid_request(format!("amount could not be parsed: {error}")))?;

        let billing = request
            .postal_code
            .clone()
            .map(|zip| api_models::payments::Address {
                address: Some(api_models::payments::AddressDetails {
                    zip: Some(zip),
                    ..Default::default()
                }),
                ..Default::default()
            });

        Ok(api_models::payments::PaymentsRequest {
            amount: Some(amount.into()),
            currency: Some(currency),
            payment_method: Some(common_enums::PaymentMethod::Card),
            payment_method_data: Some(api_models::payments::PaymentMethodDataRequest {
                payment_method_data: Some(api_models::payments::PaymentMethodData::Card(
                    api_models::payments::Card {
                        card_number: card.card_number,
                        card_exp_month: card.card_exp_month,
                        card_exp_year: card.card_exp_year,
                        card_cvc: card.card_cvc,
                        ..Default::default()
                    },
                )),
                billing: billing.clone(),
            }),
            billing,
            description: request.description.clone(),
            merchant_order_reference_id: Some(request.transaction_id.clone()),
            ..context.payments_request()
        })
    }

    fn success_response(payment: &api_models::payments::PaymentsResponse) -> Self::Response {
        TwilioGenericPayResponse::success(payment.payment_id.clone())
    }

    fn failure_response(failure: Failure) -> Self::Response {
        TwilioGenericPayResponse::failure(failure.code, failure.message)
    }
}

fn invalid_request(message: impl Into<String>) -> Failure {
    Failure::new(generic_pay::error_codes::INVALID_REQUEST, message)
}

fn extract_card(request: &TwilioGenericPayRequest) -> Result<TwilioCardDetails, Failure> {
    let missing =
        |field: &str| invalid_request(format!("{field} is required for a card charge"));

    Ok(TwilioCardDetails {
        card_number: request
            .cardnumber
            .clone()
            .ok_or_else(|| missing("cardnumber"))?,
        card_exp_month: request
            .expiry_month
            .clone()
            .ok_or_else(|| missing("expiry_month"))?,
        card_exp_year: request
            .expiry_year
            .clone()
            .ok_or_else(|| missing("expiry_year"))?,
        card_cvc: request.cvv.clone().ok_or_else(|| missing("cvv"))?,
    })
}

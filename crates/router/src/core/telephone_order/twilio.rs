use api_models::telephone_order::twilio::{
    TwilioGenericPayRequest, TwilioGenericPayResponse, TwilioPayMethod,
};
use common_utils::{
    fp_utils,
    types::{AmountConvertor, MinorUnit, StringMajorUnitForConnector},
};

use crate::core::telephone_order::{
    self, Context, Failure, Intent, Parameters, ResponseParameters, TelephoneOrderAdapter,
};

/// What Twilio can ask for that this endpoint does not do; the rest are shared.
mod error_codes {
    pub const BANK_ACCOUNT_UNSUPPORTED: &str = "BANK_ACCOUNT_UNSUPPORTED";
}

struct TwilioCardDetails {
    card_number: cards::CardNumber,
    card_exp_month: hyperswitch_masking::Secret<String>,
    card_exp_year: hyperswitch_masking::Secret<String>,
    card_cvc: hyperswitch_masking::Secret<String>,
}

/// Twilio's Generic Pay Connector. All but the request and response shapes is shared.
pub struct TwilioGenericPay;

#[async_trait::async_trait]
impl TelephoneOrderAdapter for TwilioGenericPay {
    type Request = TwilioGenericPayRequest;
    type Response = TwilioGenericPayResponse;

    fn transaction_reference(request: &Self::Request) -> String {
        request.transaction_id.clone()
    }

    /// Twilio documents `transaction_id` as stable across its own retries — the same value for
    /// all five retries of one `<Pay>` attempt, a different one for the next caller.
    fn idempotency_reference(request: &Self::Request) -> Option<&str> {
        Some(&request.transaction_id)
    }

    fn parameters(request: &Self::Request) -> Option<&Parameters> {
        request.parameters.as_ref()
    }

    /// Maps the `<Pay>` verb's fields onto a payment. Twilio's `method` picks which one.
    fn build_intent(request: &Self::Request, context: Context) -> Result<Intent, Failure> {
        fp_utils::when(
            request.bankaccountnumber.is_some() || request.routingnumber.is_some(),
            || {
                Err(Failure::new(
                    error_codes::BANK_ACCOUNT_UNSUPPORTED,
                    "Bank account payments are not supported on this endpoint",
                ))
            },
        )?;

        match request.method {
            TwilioPayMethod::Charge => build_charge(request, context).map(Intent::charge),
            TwilioPayMethod::Tokenize => build_tokenize(request, context).map(Intent::tokenize),
        }
    }

    fn charge_response(
        payment: &api_models::payments::PaymentsResponse,
        parameters: ResponseParameters,
    ) -> Self::Response {
        TwilioGenericPayResponse::charged(payment.payment_id.clone(), parameters)
    }

    fn tokenize_response(
        payment: &api_models::payments::PaymentsResponse,
        parameters: ResponseParameters,
    ) -> Result<Self::Response, Failure> {
        match payment.payment_method_id.clone() {
            Some(token_id) => Ok(TwilioGenericPayResponse::tokenized(token_id, parameters)),
            // A failure rather than a success with no token, which Twilio would carry to the
            // merchant's action URL as a tokenize that worked.
            None => Err(Failure::new(
                telephone_order::error_codes::TOKENIZE_FAILED,
                "The card was verified but could not be stored",
            )
            .with_parameters(parameters)),
        }
    }

    fn failure_response(failure: Failure) -> Self::Response {
        TwilioGenericPayResponse::failure(failure.code, failure.message, failure.parameters)
    }
}

/// Moves money. Keeps the card too when the IVR passed `setup_future_usage`.
fn build_charge(
    request: &TwilioGenericPayRequest,
    context: Context,
) -> Result<api_models::payments::PaymentsRequest, Failure> {
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

    // Rejected here rather than at the connector, which declines it with a minimum-amount
    // message naming neither the cause nor the fix.
    fp_utils::when(amount == MinorUnit::zero(), || {
        Err(invalid_request(
            "amount must be greater than zero for a charge; set chargeAmount to 0 on the <Pay> \
             verb to store a card without charging it",
        ))
    })?;

    let billing = context.billing_address(request.postal_code.clone());

    Ok(api_models::payments::PaymentsRequest {
        amount: Some(amount.into()),
        currency: Some(currency),
        payment_method: Some(common_enums::PaymentMethod::Card),
        payment_method_data: Some(payment_method_data(request, billing.clone())?),
        billing,
        description: request.description.clone(),
        merchant_order_reference_id: Some(request.transaction_id.clone()),
        ..context.payments_request()
    })
}

/// Stores the card and moves no money: a zero-amount verification at the connector.
fn build_tokenize(
    request: &TwilioGenericPayRequest,
    context: Context,
) -> Result<api_models::payments::PaymentsRequest, Failure> {
    // Twilio sends no `currency_code` on a tokenize, so the IVR supplies it. Read from
    // `currency_code` first all the same, for a provider configuration that does send one.
    let currency = request
        .currency_code
        .or_else(|| {
            request
                .parameters
                .as_ref()
                .and_then(|parameters| parameters.currency)
        })
        .ok_or_else(|| {
            invalid_request("currency is required to store a card; pass it as a <Parameter>")
        })?;

    let billing = context.billing_address(request.postal_code.clone());

    Ok(api_models::payments::PaymentsRequest {
        amount: Some(api_models::payments::Amount::Zero),
        currency: Some(currency),
        // Nothing is captured here, so a `capture_method` `<Parameter>` has no bearing.
        capture_method: Some(common_enums::CaptureMethod::Automatic),
        payment_method: Some(common_enums::PaymentMethod::Card),
        payment_method_data: Some(payment_method_data(request, billing.clone())?),
        billing,
        description: request.description.clone(),
        merchant_order_reference_id: Some(request.transaction_id.clone()),
        ..context.payments_request()
    })
}

fn payment_method_data(
    request: &TwilioGenericPayRequest,
    billing: Option<api_models::payments::Address>,
) -> Result<api_models::payments::PaymentMethodDataRequest, Failure> {
    let card = extract_card(request)?;

    Ok(api_models::payments::PaymentMethodDataRequest {
        payment_method_data: Some(api_models::payments::PaymentMethodData::Card(
            api_models::payments::Card {
                card_number: card.card_number,
                card_exp_month: card.card_exp_month,
                card_exp_year: card.card_exp_year,
                card_cvc: card.card_cvc,
                ..Default::default()
            },
        )),
        billing,
    })
}

fn invalid_request(message: impl Into<String>) -> Failure {
    Failure::new(telephone_order::error_codes::INVALID_REQUEST, message)
}

fn extract_card(request: &TwilioGenericPayRequest) -> Result<TwilioCardDetails, Failure> {
    let missing = |field: &str| invalid_request(format!("{field} is required for a card payment"));

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

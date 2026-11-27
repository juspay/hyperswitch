use api_models::telephone_order::twilio::{
    TwilioGenericPayRequest, TwilioGenericPayResponse, TwilioPayMethod,
};
use common_utils::fp_utils;

use crate::core::telephone_order::{
    self, Context, Failure, Intent, Parameters, ResponseParameters, TelephoneOrderAdapter,
};

/// What Twilio can ask for that this endpoint does not do; the rest are shared.
mod error_codes {
    pub const BANK_ACCOUNT_UNSUPPORTED: &str = "BANK_ACCOUNT_UNSUPPORTED";
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
            TwilioPayMethod::Charge => request.build_charge(context).map(Intent::charge),
            TwilioPayMethod::Tokenize => request.build_tokenize(context).map(Intent::tokenize),
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

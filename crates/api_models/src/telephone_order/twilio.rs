use std::collections::HashMap;

use hyperswitch_masking::Secret;
use utoipa::ToSchema;

/// The operation Twilio is asking us to perform. Set by the `<Pay>` verb's `chargeAmount`: `0` or
/// absent makes it a `tokenize`, any other value a `charge`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TwilioPayMethod {
    /// Move money for the collected payment method.
    Charge,
    /// Store the collected payment method for later use, without moving money.
    Tokenize,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct TwilioGenericPayRequest {
    /// The operation requested.
    #[schema(value_type = TwilioPayMethod, example = "charge")]
    pub method: TwilioPayMethod,

    /// Twilio's id for this attempt. Stable across its retries, and used to derive the `payment_id` so a retry replays the original charge.
    #[schema(example = "IP1234")]
    pub transaction_id: String,

    /// Card number collected over DTMF. Required for a charge.
    #[schema(value_type = Option<String>, example = "4242424242424242")]
    pub cardnumber: Option<cards::CardNumber>,

    /// Card security code collected over DTMF. Required for a charge.
    #[schema(value_type = Option<String>, example = "123")]
    pub cvv: Option<Secret<String>>,

    /// Two digit card expiry month. Required for a charge.
    #[schema(value_type = Option<String>, example = "12")]
    pub expiry_month: Option<Secret<String>>,

    /// Two or four digit card expiry year. Required for a charge.
    #[schema(value_type = Option<String>, example = "28")]
    pub expiry_year: Option<Secret<String>>,

    /// Billing postal code, collected for AVS.
    #[schema(value_type = Option<String>, example = "94122")]
    pub postal_code: Option<Secret<String>>,

    /// Bank account number for ACH debits. Not supported.
    #[schema(value_type = Option<String>)]
    pub bankaccountnumber: Option<Secret<String>>,

    /// Bank routing number for ACH debits. Not supported.
    #[schema(value_type = Option<String>)]
    pub routingnumber: Option<Secret<String>>,

    /// Amount in major units, as a decimal string. Absent for `tokenize`.
    #[schema(value_type = Option<String>, example = "1.00")]
    pub amount: Option<common_utils::types::StringMajorUnit>,

    /// Currency of `amount`. Absent for `tokenize`, which takes its currency from
    /// `parameters.currency` instead.
    #[schema(value_type = Option<Currency>, example = "USD")]
    pub currency_code: Option<common_enums::Currency>,

    /// Free-form description configured on the `<Pay>` verb.
    #[schema(example = "Order placed over the phone")]
    pub description: Option<String>,

    /// Values from the IVR's `<Parameter>` tags, stored unmasked in the payment's `metadata`.
    #[schema(value_type = Option<TelephoneOrderParameters>)]
    pub parameters: Option<crate::telephone_order::TelephoneOrderParameters>,
}

/// Body Twilio expects back. It reads `error_code` to decide the outcome, so a decline is a `200` with that set.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct TwilioGenericPayResponse {
    /// The Hyperswitch `payment_id`, on a successful charge. Omitted when there is no charge to
    /// point at: a `tokenize`, which answers through `token_id`, or a failure.
    #[schema(value_type = Option<String>, example = "pay_1234")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub charge_id: Option<common_utils::id_type::PaymentId>,

    /// The stored card, as a Hyperswitch `payment_method_id`: the answer to a successful
    /// `tokenize`, and sent on nothing else. A charge that kept the card reports it as
    /// `parameters.payment_method_id` instead. Charge it again as
    /// `recurring_details.payment_method_id`.
    #[schema(example = "pm_1234")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_id: Option<String>,

    /// Set when the operation did not succeed: the connector's decline code, or one of `INVALID_REQUEST`, `CUSTOMER_REQUIRED`, `ACCEPTANCE_REQUIRED`, `PROFILE_NOT_FOUND`, `PAYMENT_NOT_COMPLETED`, `PAYMENT_PENDING`, `DUPLICATE_TRANSACTION`, `PROCESSING_ERROR`, `TOKENIZE_FAILED`, `BANK_ACCOUNT_UNSUPPORTED`. On `PAYMENT_PENDING` the funds may be held, so do not re-prompt for a card.
    #[schema(example = "PAYMENT_NOT_COMPLETED")]
    pub error_code: Option<String>,

    /// Human readable counterpart to `error_code`.
    #[schema(example = "Your card was declined.")]
    pub error_message: Option<String>,

    /// Returned either way, and the only channel carrying a `payment_id` when there is no `charge_id`. Flat and all-string. The request's own `parameters` are never echoed back.
    #[schema(value_type = Object, example = json!({
        "payment_id": "pay_1234",
        "status": "succeeded",
        "amount_capturable": "0",
        "amount_received": "100",
        "connector": "stripe",
        "connector_transaction_id": "pi_1234",
        "payment_method_id": "pm_1234",
        "merchant_order_reference_id": "IP1234"
    }))]
    pub parameters: HashMap<String, String>,
}

impl TwilioGenericPayResponse {
    /// A charge that went through. No `token_id`: that is a `tokenize`'s answer, and a charge
    /// that kept the card carries it as `parameters.payment_method_id`.
    pub fn charged(
        charge_id: common_utils::id_type::PaymentId,
        parameters: HashMap<String, String>,
    ) -> Self {
        Self {
            charge_id: Some(charge_id),
            token_id: None,
            error_code: None,
            error_message: None,
            parameters,
        }
    }

    /// A card stored without a charge. No `charge_id`; the payment is in `parameters.payment_id`.
    pub fn tokenized(token_id: String, parameters: HashMap<String, String>) -> Self {
        Self {
            charge_id: None,
            token_id: Some(token_id),
            error_code: None,
            error_message: None,
            parameters,
        }
    }

    pub fn failure(
        error_code: String,
        error_message: String,
        parameters: HashMap<String, String>,
    ) -> Self {
        Self {
            charge_id: None,
            token_id: None,
            error_code: Some(error_code),
            error_message: Some(error_message),
            parameters,
        }
    }
}

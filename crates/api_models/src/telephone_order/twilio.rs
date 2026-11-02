use std::collections::HashMap;

use hyperswitch_masking::Secret;

/// The operation Twilio is asking us to perform.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TwilioPayMethod {
    /// Move money for the collected payment method.
    Charge,
    /// Store the collected payment method for later use, without moving money.
    Tokenize,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TwilioGenericPayRequest {
    /// Whether Twilio wants a charge or a tokenization.
    pub method: TwilioPayMethod,

    /// Twilio's identifier for this payment attempt. Stable across Twilio's retries.
    pub transaction_id: String,

    /// Card number collected over DTMF.
    pub cardnumber: Option<cards::CardNumber>,

    /// Card security code collected over DTMF.
    pub cvv: Option<Secret<String>>,

    /// Two digit card expiry month.
    pub expiry_month: Option<Secret<String>>,

    /// Two or four digit card expiry year.
    pub expiry_year: Option<Secret<String>>,

    /// Billing postal code, collected for AVS.
    pub postal_code: Option<Secret<String>>,

    /// Bank account number, sent for ACH debits. Not supported.
    pub bankaccountnumber: Option<Secret<String>>,

    /// Bank routing number, sent for ACH debits. Not supported.
    pub routingnumber: Option<Secret<String>>,

    /// Amount in major units, as a decimal string. Absent for `tokenize`.
    pub amount: Option<common_utils::types::StringMajorUnit>,

    /// Currency of `amount`. Absent for `tokenize`.
    pub currency_code: Option<common_enums::Currency>,

    /// Free-form description configured on the `<Pay>` verb.
    pub description: Option<String>,

    /// Values supplied by the IVR through `<Parameter>` tags on the `<Pay>` verb. Not a `Secret`:
    /// the core persists these unmasked in the payment's `metadata`, so keep nothing sensitive here.
    pub parameters: Option<HashMap<String, String>>,
}

/// Body Twilio expects back. It reads `error_code` to decide whether the payment succeeded, so a
/// decline is a `200` with that set; every field is serialized even when `None`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TwilioGenericPayResponse {
    /// Reference for a successful charge. This is the Hyperswitch `payment_id`.
    pub charge_id: Option<common_utils::id_type::PaymentId>,

    /// Set when the payment did not succeed. `null` on success.
    pub error_code: Option<String>,

    /// Human readable counterpart to `error_code`. `null` on success.
    pub error_message: Option<String>,

    /// Handed back either way — the only channel carrying a `payment_id` when `charge_id` is
    /// `null`. Flat and all-string: Twilio raises a JSON mapping exception on anything else.
    pub parameters: HashMap<String, String>,
}

impl TwilioGenericPayResponse {
    pub fn success(
        charge_id: common_utils::id_type::PaymentId,
        parameters: HashMap<String, String>,
    ) -> Self {
        Self {
            charge_id: Some(charge_id),
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
            error_code: Some(error_code),
            error_message: Some(error_message),
            parameters,
        }
    }
}

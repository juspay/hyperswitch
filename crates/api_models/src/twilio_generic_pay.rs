use std::collections::HashMap;

use hyperswitch_masking::Secret;
use utoipa::ToSchema;

/// The operation Twilio is asking us to perform.
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
    /// Whether Twilio wants a charge or a tokenization.
    #[schema(value_type = TwilioPayMethod)]
    pub method: TwilioPayMethod,

    /// Twilio's identifier for this payment attempt. Stable across Twilio's retries.
    #[schema(example = "TXN123456789")]
    pub transaction_id: String,

    /// Card number collected over DTMF.
    #[schema(value_type = Option<String>, example = "4242424242424242")]
    pub cardnumber: Option<cards::CardNumber>,

    /// Card security code collected over DTMF.
    #[schema(value_type = Option<String>, example = "123")]
    pub cvv: Option<Secret<String>>,

    /// Two digit card expiry month.
    #[schema(value_type = Option<String>, example = "10")]
    pub expiry_month: Option<Secret<String>>,

    /// Two or four digit card expiry year.
    #[schema(value_type = Option<String>, example = "35")]
    pub expiry_year: Option<Secret<String>>,

    /// Billing postal code, collected for AVS.
    #[schema(value_type = Option<String>, example = "94107")]
    pub postal_code: Option<Secret<String>>,

    /// Bank account number, sent for ACH debits. Not supported.
    #[schema(value_type = Option<String>)]
    pub bankaccountnumber: Option<Secret<String>>,

    /// Bank routing number, sent for ACH debits. Not supported.
    #[schema(value_type = Option<String>)]
    pub routingnumber: Option<Secret<String>>,

    /// Amount in major units, as a decimal string. Absent for `tokenize`.
    #[schema(value_type = Option<String>, example = "25.00")]
    pub amount: Option<common_utils::types::StringMajorUnit>,

    /// Currency of `amount`. Absent for `tokenize`.
    #[schema(value_type = Option<Currency>, example = "USD")]
    pub currency_code: Option<common_enums::Currency>,

    /// Free-form description configured on the `<Pay>` verb.
    #[schema(example = "Toll violation payment")]
    pub description: Option<String>,

    /// Values supplied by the IVR through `<Parameter>` tags on the `<Pay>` verb.
    #[schema(value_type = Option<Object>)]
    pub parameters: Option<HashMap<String, String>>,
}

/// Body Twilio expects back from the Generic Pay Connector endpoint.
///
/// Twilio reads `error_code` to decide whether the payment succeeded, so a declined payment is a
/// `200` with `error_code` populated rather than an HTTP error. Every field is serialized even when
/// `None`: Twilio expects the `charge_id` key to be present and `null` on failure, not omitted.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct TwilioGenericPayResponse {
    /// Reference for a successful charge. This is the Hyperswitch `payment_id`.
    #[schema(example = "pay_mbabizu24mvu3mela5njyhpit4")]
    pub charge_id: Option<String>,

    /// Set when the payment did not succeed. `null` on success.
    #[schema(example = "UE_9000")]
    pub error_code: Option<String>,

    /// Human readable counterpart to `error_code`. `null` on success.
    #[schema(example = "Card declined by the issuer")]
    pub error_message: Option<String>,
}

impl TwilioGenericPayResponse {
    pub fn success(charge_id: String) -> Self {
        Self {
            charge_id: Some(charge_id),
            error_code: None,
            error_message: None,
        }
    }

    pub fn failure(error_code: String, error_message: String) -> Self {
        Self {
            charge_id: None,
            error_code: Some(error_code),
            error_message: Some(error_message),
        }
    }
}

use common_enums::Currency;
use common_utils::types::StringMinorUnit;
use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::utils::deserialize_optional_currency;

#[derive(Debug, Serialize, Deserialize)]
pub struct GlobalpayPaymentsResponse {
    pub status: GlobalpayPaymentStatus,
    pub payment_method: Option<PaymentMethod>,
    pub id: String,
    pub amount: Option<StringMinorUnit>,
    #[serde(deserialize_with = "deserialize_optional_currency")]
    pub currency: Option<Currency>,
    pub reference: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GlobalpayRefreshTokenResponse {
    pub token: Secret<String>,
    pub seconds_to_expire: i64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GlobalpayRefreshTokenErrorResponse {
    pub error_code: String,
    pub detailed_error_description: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaymentMethod {
    pub apm: Option<Apm>,
    pub card: Option<Card>,
    pub id: Option<Secret<String>>,
    pub message: Option<String>,
    pub result: Option<String>,
}

/// Data associated with the response of an APM transaction.
#[derive(Debug, Serialize, Deserialize)]
pub struct Apm {
    #[serde(alias = "provider_redirect_url")]
    pub redirect_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Card {
    pub brand_reference: Option<Secret<String>>,
}

/// Indicates where a transaction is in its lifecycle.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GlobalpayPaymentStatus {
    /// A Transaction has been successfully authorized and captured. The funding
    /// process will commence once the transaction remains in this status.
    Captured,
    /// A Transaction where the payment method provider declined the transfer of
    /// funds between the payer and the merchant.
    Declined,
    /// A Transaction where the funds have transferred between payer and merchant as
    /// expected.
    Funded,
    /// A Transaction has been successfully initiated. An update on its status is
    /// expected via a separate asynchronous notification to a webhook.
    Initiated,
    /// A Transaction has been sent to the payment method provider and are waiting
    /// for a result.
    Pending,
    /// A Transaction has been approved but a capture request is required to
    /// commence the movement of funds.
    Preauthorized,
    /// A Transaction where the funds were expected to transfer between payer and
    /// merchant but the transfer was rejected during the funding process. This rarely happens
    /// but when it does it is usually addressed by Global Payments operations.
    Rejected,
    /// A Transaction that had a status of PENDING, PREAUTHORIZED or CAPTURED has
    /// subsequently been reversed which voids/cancels a transaction before it is funded.
    Reversed,
}

#[derive(Debug, Deserialize)]
pub struct GlobalpayWebhookObjectId {
    pub id: String,
}

#[derive(Debug, Deserialize)]
pub struct GlobalpayWebhookObjectEventType {
    pub status: GlobalpayWebhookStatus,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GlobalpayWebhookStatus {
    Declined,
    Captured,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GlobalpayPaymentMethodsResponse {
    #[serde(rename = "id")]
    pub payment_method_token_id: Option<Secret<String>>,
    pub card: Card,
}

use common_utils::{id_type::CustomerId, types::StringMinorUnit};
use masking::Secret;
use serde::Serialize;

pub struct FacilitapayRouterData<T> {
    pub amount: StringMinorUnit,
    pub router_data: T,
}

#[derive(Debug, Serialize)]
pub struct FacilitapayAuthRequest {
    pub user: FacilitapayCredentials,
}

#[derive(Debug, Serialize)]
pub struct FacilitapayCredentials {
    pub username: Secret<String>, // email_id
    pub password: Secret<String>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct FacilitapayCardDetails {
    #[serde(rename = "card_number")]
    pub number: cards::CardNumber,
    #[serde(rename = "card_expiration_date")]
    pub expiry_date: Secret<String>, // Format: "MM/YYYY"
    #[serde(rename = "card_security_code")]
    pub cvc: Secret<String>,
    #[serde(rename = "card_brand")]
    pub brand: String,
    pub fullname: Secret<String>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct CardTransactionRequest {
    pub currency: String,
    pub exchange_currency: String,
    pub value: StringMinorUnit,
    pub from_credit_card: FacilitapayCardDetails,
    pub to_bank_account_id: Secret<String>, // UUID
    pub subject_id: String,                 // UUID
}

#[derive(Debug, Serialize, Default, PartialEq)]
pub struct PixTransactionRequest {
    pub subject_id: CustomerId,               // UUID
    pub from_bank_account_id: Secret<String>, // UUID
    pub to_bank_account_id: Secret<String>,   // UUID
    pub currency: String,
    pub exchange_currency: String,
    pub value: StringMinorUnit,
    pub use_dynamic_pix: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dynamic_pix_expires_at: Option<String>,
}

#[derive(Debug, Serialize, PartialEq)]
pub enum FacilitapayTransactionRequest {
    #[allow(dead_code)]
    Card(CardTransactionRequest),
    Pix(PixTransactionRequest),
}

#[derive(Debug, Serialize, PartialEq)]
pub struct FacilitapayPaymentsRequest {
    pub transaction: FacilitapayTransactionRequest,
}

// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct FacilitapayRefundRequest {
    pub amount: StringMinorUnit,
}

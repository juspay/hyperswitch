use common_utils::types::StringMajorUnit;
use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::connectors::payload::responses;

#[derive(Debug, Serialize, PartialEq)]
#[serde(untagged)]
pub enum PayloadPaymentsRequest {
    PayloadCardsRequest(Box<PayloadCardsRequestData>),
    PayloadMandateRequest(Box<PayloadMandateRequestData>),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TransactionTypes {
    Credit,
    Chargeback,
    ChargebackReversal,
    Deposit,
    Payment,
    Refund,
    Reversal,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BillingAddress {
    #[serde(rename = "payment_method[billing_address][city]")]
    pub city: String,
    #[serde(rename = "payment_method[billing_address][country_code]")]
    pub country: common_enums::CountryAlpha2,
    #[serde(rename = "payment_method[billing_address][postal_code]")]
    pub postal_code: Secret<String>,
    #[serde(rename = "payment_method[billing_address][state_province]")]
    pub state_province: Secret<String>,
    #[serde(rename = "payment_method[billing_address][street_address]")]
    pub street_address: Secret<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct PayloadCardsRequestData {
    pub amount: StringMajorUnit,
    #[serde(flatten)]
    pub card: PayloadCard,
    #[serde(rename = "type")]
    pub transaction_types: TransactionTypes,
    // For manual capture, set status to "authorized", otherwise omit
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<responses::PayloadPaymentStatus>,
    #[serde(rename = "payment_method[type]")]
    pub payment_method_type: String,
    // Billing address fields are for AVS validation
    #[serde(flatten)]
    pub billing_address: BillingAddress,
    pub processing_id: Option<Secret<String>>,
    /// Allows one-time payment by customer without saving their payment method
    /// This is true by default
    #[serde(rename = "payment_method[keep_active]")]
    pub keep_active: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct PayloadMandateRequestData {
    pub amount: StringMajorUnit,
    #[serde(rename = "type")]
    pub transaction_types: TransactionTypes,
    // Based on the connectors' response, we can do recurring payment either based on a default payment method id saved in the customer profile or a specific payment method id
    // Connector by default, saves every payment method
    pub payment_method_id: Secret<String>,
    // For manual capture, set status to "authorized", otherwise omit
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<responses::PayloadPaymentStatus>,
}

#[derive(Default, Clone, Debug, Serialize, Eq, PartialEq)]
pub struct PayloadCard {
    #[serde(rename = "payment_method[card][card_number]")]
    pub number: cards::CardNumber,
    #[serde(rename = "payment_method[card][expiry]")]
    pub expiry: Secret<String>,
    #[serde(rename = "payment_method[card][card_code]")]
    pub cvc: Secret<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct PayloadCancelRequest {
    pub status: responses::PayloadPaymentStatus,
}

// Type definition for CaptureRequest
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct PayloadCaptureRequest {
    pub status: responses::PayloadPaymentStatus,
}

// Type definition for RefundRequest
#[derive(Debug, Serialize)]
pub struct PayloadRefundRequest {
    #[serde(rename = "type")]
    pub transaction_type: TransactionTypes,
    pub amount: StringMajorUnit,
    #[serde(rename = "ledger[0][assoc_transaction_id]")]
    pub ledger_assoc_transaction_id: String,
}

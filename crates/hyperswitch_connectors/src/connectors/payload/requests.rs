use common_utils::{pii::Email, types::StringMajorUnit};
use hyperswitch_masking::Secret;
use serde::{Deserialize, Serialize};

use crate::connectors::payload::responses;

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum PayloadPaymentsRequest {
    PaymentRequest(Box<PayloadPaymentRequestData>),
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

/// Billing address nested inside `payment_method` for AVS validation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BillingAddress {
    pub city: Option<String>,
    pub country_code: Option<common_enums::CountryAlpha2>,
    pub postal_code: Secret<String>,
    pub state_province: Option<Secret<String>>,
    pub street_address: Option<Secret<String>>,
}

/// Top-level payment request sent to /transactions
#[derive(Debug, Clone, Serialize)]
pub struct PayloadPaymentRequestData {
    pub amount: StringMajorUnit,
    /// Serialises as `{"type": "card"|"bank_account", "card"|"bank_account": {...},
    ///                  "billing_address": {...}, "keep_active": bool, ...}`
    pub payment_method: PayloadPaymentMethod,
    #[serde(rename = "type")]
    pub transaction_types: TransactionTypes,
    /// For manual capture, set to "authorized", otherwise omit
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<responses::PayloadPaymentStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub processing_id: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub customer_id: Option<String>,
}

/// Wrapper that nests `billing_address` and `keep_active` inside `payment_method`
#[derive(Debug, Clone, Serialize)]
pub struct PayloadPaymentMethod {
    #[serde(flatten)]
    pub method: PayloadPaymentMethods,
    /// Billing address for AVS — lives inside payment_method in the API
    #[serde(skip_serializing_if = "Option::is_none")]
    pub billing_address: Option<BillingAddress>,
    /// Whether to keep the payment method active (set false for one-time payments)
    pub keep_active: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct CustomerRequest {
    pub keep_active: bool,
    pub email: Email,
    pub name: Secret<String>,
    pub primary_processing_id: Option<Secret<String>>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct PayloadMandateRequestData {
    pub amount: StringMajorUnit,
    #[serde(rename = "type")]
    pub transaction_types: TransactionTypes,
    // Connector by default saves every payment method; reference by specific PM id for recurring
    pub payment_method_id: Secret<String>,
    // For manual capture, set status to "authorized", otherwise omit
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<responses::PayloadPaymentStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub processing_id: Option<Secret<String>>,
}

#[derive(Default, Clone, Debug, Serialize, Eq, PartialEq)]
pub struct PayloadCardData {
    pub card_number: cards::CardNumber,
    pub expiry: Secret<String>,
    pub card_code: Secret<String>,
}

#[derive(Default, Clone, Debug, Serialize, Eq, PartialEq)]
pub struct PayloadCard {
    pub card: PayloadCardData,
}

#[derive(Clone, Debug, Serialize)]
pub struct PayloadBankAccountInner {
    pub account_class: Option<PayloadAccClass>,
    pub account_currency: String,
    pub account_number: Secret<String>,
    pub account_type: PayloadAccAccountType,
    pub routing_number: Secret<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct PayloadBank {
    pub bank_account: PayloadBankAccountInner,
    pub account_holder: Secret<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PayloadAccClass {
    Personal,
    Business,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PayloadAccAccountType {
    Checking,
    Savings,
}

/// Tagged enum — serialises as `{"type": "card", "card": {...}}` or
/// `{"type": "bank_account", "bank_account": {...}}`
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PayloadPaymentMethods {
    Card(PayloadCard),
    BankAccount(PayloadBank),
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
    pub ledger: Vec<PayloadRefundLedgerEntry>,
}

#[derive(Debug, Serialize)]
pub struct PayloadRefundLedgerEntry {
    pub assoc_transaction_id: String,
}

// Request struct for ACH SetupMandate using /payment_methods API
#[derive(Debug, Clone, Serialize)]
pub struct PayloadPaymentMethodRequest {
    pub account_id: Secret<String>, // Customer ID from createCustomer
    pub bank_account: PayloadBankAccountData,
    pub account_holder: Secret<String>,
    #[serde(rename = "type")]
    pub payment_method_type: PayloadPaymentMethodType,
}

#[derive(Debug, Clone, Serialize)]
pub struct PayloadBankAccountData {
    pub account_number: Secret<String>,
    pub routing_number: Secret<String>,
    pub account_type: PayloadAccAccountType,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PayloadPaymentMethodType {
    BankAccount,
}

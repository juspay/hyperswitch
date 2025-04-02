use common_utils::{id_type::CustomerId, pii, types::StringMinorUnit};
use masking::Secret;
use serde::{Deserialize, Serialize};

// Response body for POST /sign_in
#[derive(Debug, Deserialize, Serialize)]
pub struct FacilitapayAuthResponse {
    username: String,
    name: String,
    pub jwt: Secret<String>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Subject {
    pub id: String,
    pub status: Option<String>,
    pub social_name: Option<String>,
    pub document_type: Option<String>,
    pub document_number: Option<Secret<String>>,
    pub birth_date: Option<String>,
    pub fiscal_country: Option<String>,
    pub email: Option<pii::Email>,
    pub phone_number: Option<Secret<String>>,
    pub address_street: Option<Secret<String>>,
    pub address_number: Option<Secret<String>>,
    pub address_complement: Option<Secret<String>>,
    pub address_city: Option<String>,
    pub address_state: Option<String>,
    pub address_postal_code: Option<Secret<String>>,
    pub address_country: Option<String>,
    pub net_monthly_average_income: Option<String>,
    pub clearance_level: Option<i32>,
    pub required_clearance_level: Option<i32>,
    pub inserted_at: Option<String>,
    pub updated_at: Option<String>,
    #[serde(default)]
    pub references: Option<Vec<serde_json::Value>>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OwnerCompany {
    pub id: String,
    pub social_name: String,
    pub document_type: String,
    pub document_number: Secret<String>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Bank {
    pub id: String,
    pub name: String,
    pub swift: Option<String>,
    pub code: Option<String>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PixInfo {
    #[serde(rename = "type")]
    pub key_type: String,
    pub key: Secret<String>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BankAccount {
    pub id: String,
    pub currency: String,
    pub owner_name: Option<Secret<String>>,
    pub owner_document_number: Option<Secret<String>>,
    pub owner_company: Option<OwnerCompany>,
    pub bank: Option<Bank>,
    pub account_number: Option<Secret<String>>,
    pub account_type: Option<String>,
    pub branch_number: Option<String>,
    pub branch_country: Option<String>,
    pub routing_number: Option<String>,
    pub iban: Option<Secret<String>>,
    pub aba: Option<String>,
    pub pix_info: Option<PixInfo>,
    pub intermediary_bank_account: Option<serde_json::Value>,
    pub intermediary_bank_account_id: Option<String>,
    pub internal: Option<bool>,
    pub flow_type: Option<String>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CreditCardResponseInfo {
    pub id: String,
    pub status: Option<String>,
    pub document_type: Option<String>,
    pub document_number: Option<Secret<String>>,
    pub birthdate: Option<String>,
    pub phone_country_code: Option<String>,
    pub phone_area_code: Option<String>,
    pub phone_number: Option<Secret<String>>,
    pub inserted_at: Option<String>,
}

// PaymentsResponse
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FacilitapayPaymentStatus {
    #[default]
    Pending,
    Identified,
    Exchanged,
    Wired,
    Canceled,
    #[serde(rename = "other")]
    Unknown,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FacilitapayPaymentsResponse {
    pub data: TransactionData,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TransactionData {
    pub id: String,
    pub status: FacilitapayPaymentStatus,
    pub value: StringMinorUnit,
    pub currency: String,
    pub exchange_currency: String,
    pub subject_id: CustomerId,
    pub subject: Option<Subject>,
    pub to_bank_account: Option<BankAccount>,
    pub subject_is_receiver: Option<bool>,
    pub source_name: Option<String>,
    pub source_document_type: Option<String>,
    pub source_document_number: Option<Secret<String>>,
    pub inserted_at: Option<String>,
    pub for_exchange: Option<bool>,
    pub exchange_under_request: Option<bool>,
    pub estimated_value_until_exchange: Option<bool>,
    pub cleared: Option<bool>,
    pub bank_transaction: Option<serde_json::Value>,
    pub from_credit_card: Option<CreditCardResponseInfo>,
    pub from_bank_account: Option<BankAccount>,
    pub dynamic_pix_code: Option<String>, // QR code string
    pub exchanged_value: Option<String>,
    pub meta: Option<serde_json::Value>,
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Default, Deserialize, Clone)]
pub enum RefundStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
    #[serde(other)]
    Unknown,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RefundData {
    pub id: String,
    pub status: RefundStatus,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct FacilitapayRefundResponse {
    pub data: RefundData,
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct FacilitapayErrorResponse {
    pub code: String,
    pub error: String,
    pub message: Option<String>,
}

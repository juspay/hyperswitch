use common_utils::{pii, types::StringMajorUnit};
use masking::Secret;
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use super::requests::{AddressState, DocumentType};

// Response body for POST /sign_in
#[derive(Debug, Deserialize, Serialize)]
pub struct FacilitapayAuthResponse {
    username: String,
    name: String,
    pub jwt: Secret<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SubjectKycStatus {
    // Customer is able to send/receive money through the platform. No action is needed on your side.
    Approved,
    // Customer is required to upload documents or uploaded documents have been rejected by KYC.
    Reproved,
    // Customer has uploaded KYC documents but awaiting analysis from the backoffice. No action is needed on your side.
    WaitingApproval,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct FacilitapaySubject {
    pub social_name: Secret<String>,
    pub document_type: DocumentType,
    pub document_number: Secret<String>,
    // In documentation, both CountryAlpha2 and String are used. We cannot rely on CountryAlpha2.
    pub fiscal_country: String,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub updated_at: Option<PrimitiveDateTime>,
    pub status: SubjectKycStatus,
    pub id: String, // Subject ID
    pub birth_date: Option<time::Date>,
    pub email: Option<pii::Email>,
    pub phone_country_code: Option<Secret<String>>,
    pub phone_area_code: Option<Secret<String>>,
    pub phone_number: Option<Secret<String>>,
    pub address_street: Option<Secret<String>>,
    pub address_number: Option<Secret<String>>,
    pub address_complement: Option<Secret<String>>,
    pub address_city: Option<String>,
    pub address_state: Option<AddressState>,
    pub address_postal_code: Option<Secret<String>>,
    pub address_country: Option<String>,
    pub net_monthly_average_income: Option<StringMajorUnit>,
    pub clearance_level: Option<i32>,
    pub required_clearance_level: Option<i32>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub inserted_at: Option<PrimitiveDateTime>,
    pub references: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct FacilitapaySubjectPeopleResponse {
    pub data: FacilitapaySubject,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PixInfo {
    #[serde(rename = "type")]
    pub key_type: String,
    pub key: Secret<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CreditCardResponseInfo {
    pub id: String,
    pub status: Option<String>,
    pub document_type: String,
    pub document_number: Secret<String>,
    pub birthdate: Option<String>,
    pub phone_country_code: Option<String>,
    pub phone_area_code: Option<String>,
    pub phone_number: Option<Secret<String>>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub inserted_at: Option<PrimitiveDateTime>,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FacilitapayPaymentsResponse {
    pub data: TransactionData,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OwnerCompany {
    pub social_name: Option<String>,
    pub id: Option<String>, // Subject ID
    pub document_type: DocumentType,
    pub document_number: Secret<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BankInfo {
    pub swift: Option<Secret<String>>,
    pub name: Option<String>,
    pub id: String, // Bank ID (UUID)
    pub code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BankAccountDetail {
    pub routing_number: Option<String>,
    pub pix_info: Option<PixInfo>,
    pub owner_name: Option<String>,
    pub owner_document_number: Option<Secret<String>>,
    pub owner_company: Option<OwnerCompany>,
    pub internal: Option<bool>,
    pub intermediary_bank_account: Option<String>,
    pub intermediary_bank_account_id: Option<String>,
    pub id: String, // Bank Account ID (UUID)
    pub iban: Option<String>,
    pub flow_type: Option<String>,
    pub currency: String,
    pub branch_number: Option<String>,
    pub branch_country: Option<String>,
    pub bank: Option<BankInfo>,
    pub account_type: Option<String>,
    pub account_number: Option<String>,
    pub aba: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TransactionData {
    pub id: String, // Transaction ID (UUID)
    pub value: StringMajorUnit,
    pub status: FacilitapayPaymentStatus,
    pub currency: String,
    pub exchange_currency: Option<String>,

    // Details about the destination account (Required)
    pub to_bank_account: BankAccountDetail,

    // Details about the source - mutually exclusive
    pub from_credit_card: Option<CreditCardResponseInfo>,
    pub from_bank_account: Option<BankAccountDetail>, // Populated for PIX

    // Subject information (customer)
    pub subject_id: String,
    pub subject: Option<FacilitapaySubject>,
    pub subject_is_receiver: Option<bool>,

    // Source identification (potentially redundant with subject or card/bank info)
    pub source_name: String,
    pub source_document_type: DocumentType,
    pub source_document_number: Secret<String>,

    // Timestamps and flags
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub inserted_at: Option<PrimitiveDateTime>,
    pub for_exchange: Option<bool>,
    pub exchange_under_request: Option<bool>,
    pub estimated_value_until_exchange: Option<bool>,
    pub cleared: Option<bool>,

    // PIX specific field
    pub dynamic_pix_code: Option<String>, // QR code string for PIX

    // Exchange details
    pub exchanged_value: Option<StringMajorUnit>,

    // Other fields
    pub bank_transaction: Option<serde_json::Value>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefundData {
    pub id: String,
    pub status: RefundStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FacilitapayRefundResponse {
    pub data: RefundData,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct FacilitapayErrorResponse {
    pub code: String,
    pub error: String,
    pub message: Option<String>,
}

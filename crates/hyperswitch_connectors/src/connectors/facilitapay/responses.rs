use common_utils::{pii, types::StringMajorUnit};
use masking::Secret;
use serde::{Deserialize, Serialize};

use super::requests::DocumentType;

// Response body for POST /sign_in
#[derive(Debug, Deserialize, Serialize)]
pub struct FacilitapayAuthResponse {
    username: Secret<String>,
    name: Secret<String>,
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
    pub updated_at: Option<String>,
    pub status: SubjectKycStatus,
    #[serde(rename = "id")]
    pub customer_id: Secret<String>,
    pub birth_date: Option<time::Date>,
    pub email: Option<pii::Email>,
    pub phone_country_code: Option<Secret<String>>,
    pub phone_area_code: Option<Secret<String>>,
    pub phone_number: Option<Secret<String>>,
    pub address_street: Option<Secret<String>>,
    pub address_number: Option<Secret<String>>,
    pub address_complement: Option<Secret<String>>,
    pub address_city: Option<String>,
    pub address_state: Option<String>,
    pub address_postal_code: Option<Secret<String>>,
    pub address_country: Option<String>,
    pub address_neighborhood: Option<Secret<String>>,
    pub net_monthly_average_income: Option<StringMajorUnit>,
    pub clearance_level: Option<i32>,
    pub required_clearance_level: Option<i32>,
    pub inserted_at: Option<String>,
    pub references: Option<Vec<serde_json::Value>>,
    pub rfc_pf: Option<Secret<String>>, // 13-digit RFC, specific to Mexico users
    pub documents: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct FacilitapayCustomerResponse {
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
    pub inserted_at: Option<String>,
}

// PaymentsResponse
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[derive(strum::Display)]
pub enum FacilitapayPaymentStatus {
    /// Transaction has been created but it is waiting for an incoming TED/Wire.
    /// This is the first recorded status in production mode.
    #[default]
    Pending,
    /// Incoming TED/Wire has been identified into Facilita´s bank account.
    /// When it is a deposit into an internal bank account and there is no
    /// conversion involved (BRL to BRL for instance), that is the final state.
    Identified,
    /// The conversion rate has been closed and therefore the exchanged value
    /// is defined - when it is a deposit into an internal bank account, that is the final state.
    Exchanged,
    /// The exchanged value has been wired to its destination (a real bank account) - that is also a final state.
    Wired,
    /// When for any reason the transaction cannot be concluded or need to be reversed, it is canceled.
    #[serde(rename = "canceled")]
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FacilitapayPaymentsResponse {
    pub data: TransactionData,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OwnerCompany {
    pub social_name: Option<Secret<String>>,
    #[serde(rename = "id")]
    pub company_id: Option<String>,
    pub document_type: DocumentType,
    pub document_number: Secret<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BankInfo {
    pub swift: Option<Secret<String>>,
    pub name: Option<String>,
    #[serde(rename = "id")]
    pub bank_id: Secret<String>,
    pub code: Option<Secret<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BankAccountDetail {
    pub routing_number: Option<Secret<String>>,
    pub pix_info: Option<PixInfo>,
    pub owner_name: Option<Secret<String>>,
    pub owner_document_type: Option<String>,
    pub owner_document_number: Option<Secret<String>>,
    pub owner_company: Option<OwnerCompany>,
    pub internal: Option<bool>,
    pub intermediary_bank_account: Option<Secret<String>>,
    pub intermediary_bank_account_id: Option<Secret<String>>,
    #[serde(rename = "id")]
    pub bank_account_id: Secret<String>,
    pub iban: Option<Secret<String>>,
    pub flow_type: Option<String>,
    pub currency: api_models::enums::Currency,
    pub branch_number: Option<Secret<String>>,
    pub branch_country: Option<String>,
    pub bank: Option<BankInfo>,
    pub account_type: Option<String>,
    pub account_number: Option<Secret<String>>,
    pub aba: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TransactionData {
    #[serde(rename = "id")]
    pub transaction_id: String,
    pub value: StringMajorUnit,
    pub status: FacilitapayPaymentStatus,
    pub currency: api_models::enums::Currency,
    pub exchange_currency: api_models::enums::Currency,

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
    pub source_name: Option<Secret<String>>,
    pub source_document_type: DocumentType,
    pub source_document_number: Secret<String>,

    // Timestamps and flags
    pub inserted_at: Option<String>,
    pub for_exchange: Option<bool>,
    pub exchange_under_request: Option<bool>,
    pub estimated_value_until_exchange: Option<bool>,
    pub cleared: Option<bool>,

    // PIX specific field
    pub dynamic_pix_code: Option<String>, // QR code string for PIX

    // Exchange details
    pub exchanged_value: Option<StringMajorUnit>,

    // Cancelation details
    #[serde(rename = "canceled_reason")]
    pub cancelled_reason: Option<String>,
    #[serde(rename = "canceled_at")]
    pub cancelled_at: Option<String>,

    // Other fields
    pub bank_transaction: Option<Secret<serde_json::Value>>,
    pub meta: Option<serde_json::Value>,
}

// Void response structures (for /refund endpoint)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoidBankTransaction {
    #[serde(rename = "id")]
    pub transaction_id: String,
    pub value: StringMajorUnit,
    pub currency: api_models::enums::Currency,
    pub iof_value: Option<StringMajorUnit>,
    pub fx_value: Option<StringMajorUnit>,
    pub exchange_rate: Option<StringMajorUnit>,
    pub exchange_currency: api_models::enums::Currency,
    pub exchanged_value: StringMajorUnit,
    pub exchange_approved: bool,
    pub wire_id: Option<String>,
    pub exchange_id: Option<String>,
    pub movement_date: String,
    pub source_name: Secret<String>,
    pub source_document_number: Secret<String>,
    pub source_document_type: String,
    pub source_id: String,
    pub source_type: String,
    pub source_description: String,
    pub source_bank: Option<String>,
    pub source_branch: Option<String>,
    pub source_account: Option<String>,
    pub source_bank_ispb: Option<String>,
    pub company_id: String,
    pub company_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoidData {
    #[serde(rename = "id")]
    pub void_id: String,
    pub reason: Option<String>,
    pub inserted_at: String,
    pub status: FacilitapayPaymentStatus,
    pub transaction_kind: String,
    pub bank_transaction: VoidBankTransaction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FacilitapayVoidResponse {
    pub data: VoidData,
}

// Refund response uses the same TransactionData structure as payments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FacilitapayRefundResponse {
    pub data: TransactionData,
}

// Webhook structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FacilitapayWebhookNotification {
    pub notification: FacilitapayWebhookBody,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct FacilitapayWebhookBody {
    #[serde(rename = "type")]
    pub event_type: FacilitapayWebhookEventType,
    pub secret: Secret<String>,
    #[serde(flatten)]
    pub data: FacilitapayWebhookData,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FacilitapayWebhookEventType {
    ExchangeCreated,
    Identified,
    PaymentApproved,
    PaymentExpired,
    PaymentFailed,
    PaymentRefunded,
    WireCreated,
    WireWaitingCorrection,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum FacilitapayWebhookErrorCode {
    /// Creditor account number invalid or missing (branch_number or account_number incorrect)
    Ac03,
    /// Creditor account type missing or invalid (account_type incorrect)
    Ac14,
    /// Value in Creditor Identifier is incorrect (owner_document_number incorrect)
    Ch11,
    /// Transaction type not supported/authorized on this account (account rejected the payment)
    Ag03,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FacilitapayWebhookData {
    CardPayment {
        transaction_id: String,
        checkout_id: Option<String>,
    },
    Exchange {
        exchange_id: String,
        transaction_ids: Vec<String>,
    },
    Transaction {
        transaction_id: String,
    },
    Wire {
        wire_id: String,
        transaction_ids: Vec<String>,
    },
    WireError {
        error_code: FacilitapayWebhookErrorCode,
        error_description: String,
        bank_account_owner_id: String,
        bank_account_id: String,
        transaction_ids: Vec<String>,
        wire_id: String,
    },
}

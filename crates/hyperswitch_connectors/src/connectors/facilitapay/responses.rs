use api_models::payments::CustomerIdentificationDocumentType;
use common_utils::{pii, types::StringMajorUnit};
use masking::Secret;
use serde::{Deserialize, Serialize};

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
    pub document_type: CustomerIdentificationDocumentType,
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
    pub inserted_at: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_type: Option<CustomerIdentificationDocumentType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_number: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub birthdate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone_country_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone_area_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone_number: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub masked_card_number: Option<String>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub social_name: Option<Secret<String>>,
    #[serde(rename = "id")]
    pub company_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_type: Option<CustomerIdentificationDocumentType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_number: Option<Secret<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BankInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub swift: Option<Secret<String>>,
    pub name: Option<String>,
    #[serde(rename = "id")]
    pub bank_id: Secret<String>,
    /// Bank code in Brazil (3 digits). You can also use the ispb field below to identify a bank instead of the code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<Secret<String>>,
    /// An 8-digit code that identifies the bank in the Brazilian payments system. It can dismiss the use of the 3-digit code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ispb: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BankAccountDetail {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub routing_number: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pix_info: Option<PixInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_name: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_document_type: Option<CustomerIdentificationDocumentType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_document_number: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_company: Option<OwnerCompany>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nickname: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub internal: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intermediary_bank_account: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intermediary_bank_account_id: Option<Secret<String>>,
    #[serde(rename = "id")]
    pub bank_account_id: Secret<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iban: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flow_type: Option<String>,
    pub currency: api_models::enums::Currency,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch_number: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch_country: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bank: Option<BankInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_number: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aba: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FacilitapayPreProcessAdiqTokenResponse {
    /// The token is used to setup Cardinal SDK
    pub jwt: Secret<String>,
    /// code3DS is the code that will be used to identify the transaction in the 3DS flow
    #[serde(rename = "orderNumber")]
    pub order_number: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ThreeDsChallenge {
    pub three_ds_version: String,
    /// CReq for the ACS
    pub pareq: Secret<String>,
    /// threeDSServerTransID
    pub authentication_transaction_id: Secret<String>,
    pub acs_url: String,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_credit_card: Option<CreditCardResponseInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_bank_account: Option<BankAccountDetail>, // Populated for PIX

    // Subject information (customer)
    pub subject_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<FacilitapaySubject>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject_is_receiver: Option<bool>,

    // Source identification (potentially redundant with subject or card/bank info)
    pub source_name: Secret<String>,
    pub source_document_type: CustomerIdentificationDocumentType,
    pub source_document_number: Secret<String>,

    // Timestamps and flags
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inserted_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub for_exchange: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exchange_under_request: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_value_until_exchange: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cleared: Option<bool>,

    // PIX specific field
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dynamic_pix_code: Option<String>, // QR code string for PIX

    // Exchange details
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exchanged_value: Option<StringMajorUnit>,

    // Cancelation details
    #[serde(rename = "canceled_reason")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cancelled_reason: Option<String>,
    #[serde(rename = "canceled_at")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cancelled_at: Option<String>,

    // 3DS Challenge data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub threeds_challenge: Option<ThreeDsChallenge>,

    // Other fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bank_transaction: Option<Secret<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefundData {
    #[serde(rename = "id")]
    pub refund_id: String,
    pub status: FacilitapayPaymentStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FacilitapayRefundResponse {
    pub data: RefundData,
}

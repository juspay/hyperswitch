use common_enums::CountryAlpha2;
use common_utils::{pii, types::StringMajorUnit};
use masking::Secret;
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

pub struct FacilitapayRouterData<T> {
    pub amount: StringMajorUnit,
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
    pub value: StringMajorUnit,
    pub from_credit_card: FacilitapayCardDetails,
    pub to_bank_account_id: Secret<String>, // UUID
    pub subject_id: String,                 // UUID
}

#[derive(Debug, Serialize, PartialEq)]
pub struct PixTransactionRequest {
    pub subject_id: String,                   // UUID
    pub from_bank_account_id: Secret<String>, // UUID
    pub to_bank_account_id: Secret<String>,   // UUID
    pub currency: String,
    pub exchange_currency: String,
    pub value: StringMajorUnit,
    pub use_dynamic_pix: bool,
    #[serde(default, with = "common_utils::custom_serde::iso8601")]
    pub dynamic_pix_expires_at: PrimitiveDateTime,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(untagged)]
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
    pub amount: StringMajorUnit,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FacilitapaySubjectPeopleRequest {
    pub person: FacilitapayPerson,
}

#[derive(Debug, Serialize, Clone, Deserialize, PartialEq, Eq)]
pub enum AddressState {
    AC,
    AL,
    AP,
    AM,
    BA,
    CE,
    DF,
    ES,
    GO,
    MA,
    MT,
    MS,
    MG,
    PA,
    PB,
    PR,
    PE,
    PI,
    RJ,
    RN,
    RS,
    RO,
    RR,
    SC,
    SP,
    SE,
    TO,
}

#[derive(Debug, Serialize, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DocumentType {
    Cc,
    Cnpj,
    Cpf,
    Curp,
    Nit,
    Passport,
    Rfc,
    Rut,
    TaxId,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FacilitapayPerson {
    pub document_number: Secret<String>,
    pub document_type: DocumentType,
    pub social_name: Secret<String>,
    pub fiscal_country: CountryAlpha2,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<pii::Email>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub birth_date: Option<time::Date>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone_country_code: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone_area_code: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone_number: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address_city: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address_state: Option<AddressState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address_complement: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address_country: Option<CountryAlpha2>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address_number: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address_postal_code: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address_street: Option<Secret<String>>,
    pub net_monthly_average_income: Option<StringMajorUnit>,
}

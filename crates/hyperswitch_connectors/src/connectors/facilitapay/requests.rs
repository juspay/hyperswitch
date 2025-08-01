use common_enums::CountryAlpha2;
use common_utils::{new_type::MaskedBankAccount, pii, types::StringMajorUnit};
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
    pub currency: api_models::enums::Currency,
    pub exchange_currency: api_models::enums::Currency,
    pub value: StringMajorUnit,
    pub from_credit_card: FacilitapayCardDetails,
    pub to_bank_account_id: Secret<String>, // UUID
    pub subject_id: String,                 // UUID
}

#[derive(Debug, Serialize, PartialEq)]
pub struct PixTransactionRequest {
    pub subject_id: Secret<String>,              // Customer ID (UUID)
    pub from_bank_account_id: MaskedBankAccount, // UUID
    pub to_bank_account_id: MaskedBankAccount,   // UUID
    pub currency: api_models::enums::Currency,
    pub exchange_currency: api_models::enums::Currency,
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

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct FacilitapayCustomerRequest {
    pub person: FacilitapayPerson,
}

#[derive(Debug, Serialize, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DocumentType {
    /// CC is the Cedula de Ciudadania, is a 10-digit number, which is the national identity card for Colombian citizens.
    /// It is used for citizen identification purposes.
    #[serde(rename = "cc")]
    CedulaDeCiudadania,
    /// CNPJ stands for Cadastro Nacional da Pessoa Jurídica, is a 14-digit number,
    /// which is the national registry of legal entities in Brazil used as a unique identifier for Brazilian companies.
    #[serde(rename = "cnpj")]
    CadastroNacionaldaPessoaJurídica,
    /// CPF stands for Cadastro de Pessoas Físicas, is a 11-digit number,
    /// which is the national registry of natural persons in Brazil used as a unique identifier for Brazilian citizens.
    #[serde(rename = "cpf")]
    CadastrodePessoasFísicas,
    /// CURP stands for Clave Única de Registro de Población,is a 18-digit number used as a unique identifier for Mexican citizens.
    /// It is used to track tax information and other identification purposes by the government.
    #[serde(rename = "curp")]
    ClaveÚnicadeRegistrodePoblación,
    /// NIT is the Número de Identificación Tributaria, is a 10-digit number, which is the tax identification number in Colombia. Used for companies.
    #[serde(rename = "nit")]
    NúmerodeIdentificaciónTributaria,
    /// Passport is the travel document usually issued by a country's government
    Passport,
    /// RFC stands for Registro Federal de Contribuyentes, is a 13-digit number used as a unique identifier for Mexican companies.
    #[serde(rename = "rfc")]
    RegistroFederaldeContribuyentes,
    /// RUT stands for Rol Unico Tributario, is a 9-digit number used as a unique identifier for Chilean citizens and companies.
    #[serde(rename = "rut")]
    RolUnicoTributario,
    /// A Taxpayer Identification Number is an identifying number used for tax purposes
    TaxId,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
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
    pub address_state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address_complement: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address_country: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address_number: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address_postal_code: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address_street: Option<Secret<String>>,
    pub net_monthly_average_income: Option<StringMajorUnit>,
}

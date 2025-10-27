use common_utils::types::{FloatMajorUnit, StringMajorUnit};
use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::connectors::santander::responses;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderBoletoUpdateRequest {
    #[serde(skip_deserializing)]
    pub covenant_code: String,
    #[serde(skip_deserializing)]
    pub bank_number: String,
    pub due_date: Option<String>,
    pub discount: Option<Discount>,
    pub min_value_or_percentage: Option<f64>,
    pub max_value_or_percentage: Option<f64>,
    pub interest: Option<InterestPercentage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InterestPercentage {
    pub interest_percentage: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Discount {
    #[serde(rename = "type")]
    pub discount_type: DiscountType,
    pub discount_one: Option<DiscountObject>,
    pub discount_two: Option<DiscountObject>,
    pub discount_three: Option<DiscountObject>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub struct DiscountObject {
    pub value: f64,
    pub limit_date: String, // YYYY-MM-DD
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DiscountType {
    #[serde(rename = "ISENTO")]
    Free,
    #[serde(rename = "VALOR_DATA_FIXA")]
    FixedDateValue,
    #[serde(rename = "VALOR_DIA_CORRIDO")]
    ValueDayConductor,
    #[serde(rename = "VALOR_DIA_UTIL")]
    ValueWorthDay,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderMetadataObject {
    pub pix_key: Secret<String>,
    pub cpf: Secret<String>, // req in scheduled type pix      // 11 characters at max
    pub cnpj: Secret<String>, // req in immediate type pix      // 14 characters at max
    pub merchant_city: String,
    pub merchant_name: String,
    pub workspace_id: String,
    pub covenant_code: String, // max_size : 9
}

pub struct SantanderRouterData<T> {
    pub amount: StringMajorUnit,
    pub router_data: T,
}

#[derive(Debug, Serialize)]
pub struct SantanderAuthType {
    pub(super) client_id: Secret<String>,
    pub(super) client_secret: Secret<String>,
    pub(super) certificate: Secret<String>,
    pub(super) certificate_key: Secret<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SantanderGrantType {
    ClientCredentials,
}

#[derive(Debug, Serialize)]
pub struct SantanderAuthRequest {
    pub client_id: Secret<String>,
    pub client_secret: Secret<String>,
    pub grant_type: SantanderGrantType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProtestType {
    #[serde(rename = "SEM_PROTESTO")]
    WithoutProtest,
    #[serde(rename = "DIAS_CORRIDOS")]
    DaysConducted,
    #[serde(rename = "DIAS_UTEIS")]
    WorkingDays,
    #[serde(rename = "CADASTRO_CONVENIO")]
    RegistrationAgreement,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderDebtor {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cnpj: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpf: Option<Secret<String>>,
    #[serde(rename = "nome")]
    pub name: Secret<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "logradouro")]
    pub street: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "cidade")]
    pub city: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "uf")]
    pub state: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "cep")]
    pub zip_code: Option<Secret<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SantanderValue {
    pub original: StringMajorUnit,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SantanderPixRequestCalendar {
    Immediate(SantanderPixImmediateCalendarRequest),
    Scheduled(SantanderPixDueDateCalendarRequest),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SantanderPixImmediateCalendarRequest {
    #[serde(rename = "expiracao")]
    pub expiration: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SantanderPixDueDateCalendarRequest {
    #[serde(rename = "dataDeVencimento")]
    pub expiration_date: String,
    #[serde(rename = "validadeAposVencimento")]
    pub validity_after_expiration: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderPixCancelRequest {
    pub status: Option<responses::SantanderVoidStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SantanderPaymentsCancelRequest {
    PixQR(SantanderPixCancelRequest),
    Boleto(SantanderBoletoCancelRequest),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderBoletoCancelRequest {
    pub covenant_code: String,
    pub bank_number: String,
    pub operation: SantanderBoletoCancelOperation,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub enum SantanderBoletoCancelOperation {
    #[serde(rename = "PROTESTAR")]
    Protest,
    #[serde(rename = "CANCELAR_PROTESTO")]
    CancelProtest,
    #[serde(rename = "BAIXAR")]
    #[default]
    WriteOff,
}

#[derive(Default, Debug, Serialize)]
pub struct SantanderRefundRequest {
    #[serde(rename = "valor")]
    pub value: StringMajorUnit,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum SantanderPaymentRequest {
    PixQR(Box<SantanderPixQRPaymentRequest>),
    Boleto(Box<SantanderBoletoPaymentRequest>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SantanderPixQRPaymentRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "calendario")]
    pub calendar: Option<SantanderPixRequestCalendar>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "devedor")]
    pub debtor: Option<SantanderDebtor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "valor")]
    pub value: Option<SantanderValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "chave")]
    pub key: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "solicitacaoPagador")]
    pub request_payer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "infoAdicionais")]
    pub additional_info: Option<Vec<responses::SantanderAdditionalInfo>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderBoletoPaymentRequest {
    pub environment: Environment,
    pub nsu_code: String,
    pub nsu_date: String,
    pub covenant_code: String,
    pub bank_number: Secret<String>,
    pub client_number: Option<common_utils::id_type::CustomerId>,
    pub due_date: String,
    pub issue_date: String,
    pub nominal_value: StringMajorUnit,
    pub participant_code: Option<String>,
    pub payer: responses::Payer,
    pub beneficiary: Option<responses::Beneficiary>,
    pub document_kind: responses::BoletoDocumentKind,
    pub discount: Option<Discount>,
    pub fine_percentage: Option<String>,
    pub fine_quantity_days: Option<String>,
    pub interest_percentage: Option<String>,
    pub deduction_value: Option<FloatMajorUnit>,
    pub protest_type: Option<ProtestType>,
    pub protest_quantity_days: Option<i64>,
    pub write_off_quantity_days: Option<String>,
    pub payment_type: responses::PaymentType,
    pub parcels_quantity: Option<i64>,
    pub value_type: Option<String>,
    pub min_value_or_percentage: Option<f64>,
    pub max_value_or_percentage: Option<f64>,
    pub iof_percentage: Option<f64>,
    pub sharing: Option<responses::Sharing>,
    pub key: Option<responses::Key>,
    pub tx_id: Option<String>,
    pub messages: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Environment {
    #[serde(rename = "Teste")]
    Sandbox,
    #[serde(rename = "Producao")]
    Production,
}

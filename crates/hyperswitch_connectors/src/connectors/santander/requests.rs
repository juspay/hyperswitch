use common_utils::types::{FloatMajorUnit, StringMajorUnit};
use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::connectors::santander::responses;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderBoletoUpdateRequest {
    #[serde(skip_deserializing)]
    pub covenant_code: Secret<String>,
    #[serde(skip_deserializing)]
    pub bank_number: Secret<String>,
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
    // Free
    Isento,
    // Fixed Date Value
    ValorDataFixa,
    // Value Day Conductor
    ValorDiaCorrido,
    // Value Worth Day
    ValorDiaUtil,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SantanderMetadataObject {
    pub pix: Option<PixMetadataObject>,
    pub boleto: Option<BoletoMetadataObject>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BoletoMetadataObject {
    pub client_id: Secret<String>,
    pub client_secret: Secret<String>,
    pub cpf: Secret<String>, // req in scheduled type pix      // 11 characters at max
    pub cnpj: Secret<String>, // req in immediate type pix      // 14 characters at max
    pub workspace_id: String,
    pub covenant_code: Secret<String>, // max_size : 9
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PixMetadataObject {
    pub client_id: Secret<String>,
    pub client_secret: Secret<String>,
    pub pix_key: Secret<String>,
    pub cpf: Secret<String>, // req in scheduled type pix      // 11 characters at max
    pub cnpj: Secret<String>, // req in immediate type pix      // 14 characters at max
    pub merchant_name: String,
    pub merchant_city: String,
}

pub struct SantanderRouterData<T> {
    pub amount: StringMajorUnit,
    pub router_data: T,
}

#[derive(Debug, Serialize)]
pub struct SantanderAuthType {
    pub(super) client_id: Secret<String>,
    pub(super) client_secret: Secret<String>,
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
    // Without Protest
    SemProtesto,
    // Days Conducted
    DiasCorridos,
    // Working Days
    DiasUteis,
    // Registration Agreement
    CadastroConvenio,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderDebtor {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cnpj: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpf: Option<Secret<String>>,
    // Name
    pub nome: Secret<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    // Street
    pub logradouro: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    // City
    pub cidade: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    // State
    pub uf: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    // Zip Code
    pub cep: Option<Secret<String>>,
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
    // expiration
    pub expiracao: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderPixDueDateCalendarRequest {
    // Expiration Date
    pub data_de_vencimento: String,
    // Validity After Expiration
    pub validade_apos_vencimento: Option<i32>,
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
    pub covenant_code: Secret<String>,
    pub bank_number: Secret<String>,
    pub operation: SantanderBoletoCancelOperation,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SantanderBoletoCancelOperation {
    // Protest
    Protestar,
    // Cancel Protest
    CancelarProtesto,
    #[default]
    // Write Off
    Baixar,
}

#[derive(Default, Debug, Serialize)]
pub struct SantanderRefundRequest {
    // value
    pub valor: StringMajorUnit,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum SantanderPaymentRequest {
    PixQR(Box<SantanderPixQRPaymentRequest>),
    Boleto(Box<SantanderBoletoPaymentRequest>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename = "camelCase")]
pub struct SantanderPixQRPaymentRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    // calendar
    pub calendario: Option<SantanderPixRequestCalendar>,
    #[serde(skip_serializing_if = "Option::is_none")]
    // debtor
    pub devedor: Option<SantanderDebtor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    // value
    pub valor: Option<SantanderValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    // key
    pub chave: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    // request_payer
    pub solicitacao_pagador: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    // additional_info
    pub info_adicionais: Option<Vec<responses::SantanderAdditionalInfo>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderBoletoPaymentRequest {
    pub environment: Environment,
    pub nsu_code: String,
    pub nsu_date: String,
    pub covenant_code: Secret<String>,
    pub bank_number: Secret<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_number: Option<common_utils::id_type::CustomerId>,
    pub due_date: String,
    pub issue_date: String,
    pub nominal_value: StringMajorUnit,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub participant_code: Option<String>,
    pub payer: responses::Payer,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub beneficiary: Option<responses::Beneficiary>,
    pub document_kind: responses::BoletoDocumentKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discount: Option<Discount>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fine_percentage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fine_quantity_days: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interest_percentage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deduction_value: Option<FloatMajorUnit>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protest_type: Option<ProtestType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protest_quantity_days: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub write_off_quantity_days: Option<String>,
    pub payment_type: responses::PaymentType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parcels_quantity: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_value_or_percentage: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_value_or_percentage: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iof_percentage: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sharing: Option<responses::Sharing>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<responses::Key>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tx_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub messages: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Environment {
    // Sandbox
    Teste,
    // Production
    Producao,
}

use common_utils::types::{FloatMajorUnit, StringMajorUnit};
use hyperswitch_masking::Secret;
use serde::{Deserialize, Serialize};

use crate::connectors::santander::responses;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
// Only due date is updatable as of now. Will add other fields when required.
pub struct SantanderBoletoUpdateRequest {
    #[serde(skip_deserializing)]
    pub covenant_code: Secret<String>,
    #[serde(skip_deserializing)]
    pub bank_number: String,
    pub due_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InterestPercentage {
    pub interest_percentage: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Discount {
    #[serde(rename = "type")]
    pub discount_type: SantanderDiscountType,
    pub discount_one: Option<DiscountObject>,
    pub discount_two: Option<DiscountObject>,
    pub discount_three: Option<DiscountObject>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscountObject {
    pub value: Option<StringMajorUnit>,
    #[serde(default, with = "common_utils::custom_serde::date_only_optional")]
    pub limit_date: Option<time::PrimitiveDateTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SantanderDiscountType {
    // No discount
    Isento,
    // If the payer pays before a certain date, they get a fixed discount amount
    ValorDataFixa,
    // This gives a discount per day of early payment, counting every day(weekends included). Example : $1.50 discount for each day before due date.
    ValorDiaCorrido,
    // Same as above, but only counts business days (Mon–Fri, excluding holidays). Example : $2 off per business day of early payment.
    ValorDiaUtil,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SantanderMetadataObject {
    pub pix: Option<PixMetadataObject>,
    pub boleto: Option<BoletoMetadataObject>,
    pub pix_automatico_push: Option<PixAutomaticoMetadataObject>,
    pub pix_automatico_qr: Option<PixAutomaticoMetadataObject>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BoletoMetadataObject {
    pub client_id: Secret<String>,
    pub client_secret: Secret<String>,
    pub workspace_id: Secret<String>,
    pub covenant_code: Secret<String>,
    pub pix_key_value: Option<Secret<String>>,
    pub pix_key_type: Option<responses::SantanderPixKeyType>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PixMetadataObject {
    pub client_id: Secret<String>,
    pub client_secret: Secret<String>,
    pub pix_key_value: Secret<String>,
    pub pix_key_type: responses::SantanderPixKeyType,
    pub merchant_name: String,
    pub merchant_city: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PixAutomaticoMetadataObject {
    pub client_id: Secret<String>,
    pub client_secret: Secret<String>,
    // might not be req for J1/J2, cross check
    pub pix_key_value: Secret<String>,
    // might not be req for J1/J2, cross check
    pub pix_key_type: responses::SantanderPixKeyType,
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
pub enum SantanderProtestType {
    // No protest
    SemProtesto,
    // Protest after X calendar days
    DiasCorridos,
    // Protest after X business days
    DiasUteis,
    // No need to provide days, uses bank’s pre-registered setting
    CadastroConvenio,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderDebtor {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cnpj: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpf: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nome: Option<Secret<String>>,
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
    // expiration time in seconds
    pub expiracao: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderPixDueDateCalendarRequest {
    // Expiration Date
    pub data_de_vencimento: String,
    // Validity After Expiration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validade_apos_vencimento: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderPixAutomaticCalendarRequest {
    pub data_expiracao_solicitacao: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderPixAutomaticDestinationRequest {
    pub agencia: String,
    pub conta: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpf: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cnpj: Option<Secret<String>>,
    pub ispb_participante: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderPixAutomaticSolicitationRequest {
    pub id_rec: Secret<String>,
    pub calendario: SantanderPixAutomaticCalendarRequest,
    pub destinatario: SantanderPixAutomaticDestinationRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SantanderPostProcessingStepRequest {
    PixAutomaticoPush(SantanderPixAutomaticSolicitationRequest),
    PixAutomaticoQr(),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderPixCancelRequest {
    pub status: Option<responses::SantanderVoidStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SantanderPaymentsCancelRequest {
    PixQR(SantanderPixCancelRequest),
    Boleto(SantanderBoletoCancelRequest),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderBoletoCancelRequest {
    pub covenant_code: Secret<String>,
    pub bank_number: String,
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
    PixAutomaticoCobr(Box<SantanderPixAutomaticoCobrRequest>),
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
    // A unique identifier that links a user’s bank account and allows others to send money without needing bank details. Instead of sharing: Bank name/Branch/Account number, one can just share the chave Pix, and the Central Bank of Brazil resolves it to the correct account.
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment: Option<Environment>,
    // This is a unique identifier for the boleto registration request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nsu_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nsu_date: Option<String>,
    // It is a number which shows a contract between merchant and bank
    #[serde(skip_serializing_if = "Option::is_none")]
    pub covenant_code: Option<Secret<String>>,
    // It is a unique ID which the merchant makes to identify each txn and the bank uses this to identify unique txn's
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bank_number: Option<String>,
    // It is a unique ID which the merchant uses internally to identify each order
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_number: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issue_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nominal_value: Option<StringMajorUnit>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub participant_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payer: Option<responses::Payer>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub beneficiary: Option<responses::Beneficiary>,
    // It tells the bank what type of commercial document created the boleto. Why does this boleto exist? What kind of transaction or contract caused it?
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_kind: Option<responses::SantanderBoletoDocumentKind>,
    // The discount field indicates if the boleto gives the payer a discount for paying early
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discount: Option<Discount>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fine_percentage: Option<StringMajorUnit>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fine_quantity_days: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interest_percentage: Option<StringMajorUnit>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deduction_value: Option<FloatMajorUnit>,
    // Protest is a formal step a bank or notary office takes to claim unpaid boletos after the due date
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protest_type: Option<SantanderProtestType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protest_quantity_days: Option<String>,
    // This field tells the bank after how many days past the due date the boleto should be automatically “written off”
    #[serde(skip_serializing_if = "Option::is_none")]
    pub write_off_quantity_days: Option<String>,
    // This field tells the bank how the boleto can be paid — whether the payer must pay the exact amount, can pay a different amount, or pay in parts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_type: Option<responses::SantanderBoletoPaymentType>,
    // This becomes a required field if payment_type is Parcial. This field indicates the number of payments allowed for the same payment slip, with a maximum of 99.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parcels_quantity: Option<u32>,
    // The valueType field defines how the min/max limits are expressed for boletos that allow flexible payments. Only used if paymentType is DIVERGENTE or PARCIAL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_type: Option<SantanderValueType>,
    // This field defines the minimum amount or minimum percentage the payer can pay for a boleto that allows DIVERGENTE or PARCIAL payments.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_value_or_percentage: Option<StringMajorUnit>,
    // This field defines the max amount or max percentage the payer can pay for a boleto that allows DIVERGENTE or PARCIAL payments.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_value_or_percentage: Option<StringMajorUnit>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iof_percentage: Option<StringMajorUnit>,
    // This feature allows the merchant (beneficiário) to split the funds received from a boleto into up to four Santander accounts that they own.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sharing: Option<Vec<responses::Sharing>>,
    // This field indicates the type of PIX key that the beneficiary (merchant) has registered in Santander.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<responses::Key>,
    // The transaction id of the QR Code payment request
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tx_id: Option<String>,
    // Messages to be printed on the payment slip or the payer's receipt. They should be sent in list format with up to 45 texts of 100 characters each.
    // Example : ["Payable at any bank until the due date.", "After the due date, only at Santander branches."]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub messages: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum SantanderValueType {
    // Percentage
    Percentual,
    // Value terms
    Valor,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Environment {
    // Sandbox
    Teste,
    // Production
    Producao,
}

pub type BoletoAdditionalFields = (
    (
        Option<responses::Beneficiary>,
        Option<Discount>,
        Option<responses::SantanderBoletoDocumentKind>,
    ),
    (
        Option<StringMajorUnit>,
        Option<String>,
        Option<StringMajorUnit>,
        Option<StringMajorUnit>,
    ),
    (Option<SantanderProtestType>, Option<String>, Option<String>),
    (
        Option<responses::SantanderBoletoPaymentType>,
        Option<SantanderValueType>,
        Option<u32>,
        Option<StringMajorUnit>,
        Option<StringMajorUnit>,
    ),
);

// SetupMandate (Pix Automatico) Request Structures
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderSetupMandateRequest {
    /// Information about the recurring object/service
    pub vinculo: RecurrenceLink,
    /// Calendar information for the recurrence
    pub calendario: RecurrenceCalendar,
    /// Retry policy for failed payments
    pub politica_retentativa: RetryPolicy,
    /// Location ID for QR code
    #[serde(skip_serializing_if = "Option::is_none")]
    pub loc: Option<i64>,
    /// Value information (optional, for fixed amount mandates)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub valor: Option<RecurrenceValue>,
    /// Activation data (required for Journey 3)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ativacao: Option<RecurrenceActivation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecurrenceValue {
    /// Fixed recurring value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub valor_rec: Option<StringMajorUnit>,
    /// Minimum value set by receiver
    #[serde(skip_serializing_if = "Option::is_none")]
    pub valor_minimo_recebedor: Option<StringMajorUnit>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecurrenceActivation {
    /// Journey-specific data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dados_jornada: Option<JourneyData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JourneyData {
    /// Transaction ID for Journey 3
    pub txid: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecurrenceLink {
    /// Contract identifier
    pub contrato: String,
    /// Debtor information
    pub devedor: RecurrenceDebtor,
    /// Description of the recurring object
    pub objeto: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecurrenceDebtor {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cnpj: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpf: Option<Secret<String>>,
    pub nome: Secret<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecurrenceCalendar {
    /// Initial date in YYYY-MM-DD format
    pub data_inicial: String,
    /// Periodicity of the recurrence
    pub periodicidade: Periodicidade,
    /// Optional end date in YYYY-MM-DD format
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_final: Option<String>,
}

/// Periodicity of recurring payments
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Periodicidade {
    /// Weekly
    Semanal,
    /// Monthly
    Mensal,
    /// Quarterly
    Trimestral,
    /// Semi-annually (every 6 months)
    Semestral,
    /// Annually
    Anual,
}

/// Retry policy for failed recurring payments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RetryPolicy {
    /// Does not allow retries
    #[serde(rename = "NAO_PERMITE")]
    NaoPermite,
    /// Allows 3 retries over 7 days
    #[serde(rename = "PERMITE_3R_7D")]
    Permite3r7d,
}

/// Represents receiver details for Pix Automático recurring charge (cobr endpoint)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderPixAutomaticoRecebedor {
    /// Branch code (agencia) of the receiver's account
    pub agencia: Secret<String>,
    /// Account number (conta) of the receiver
    pub conta: Secret<String>,
    /// Account type (tipoConta) - CORRENTE, POUPANCA, or PAGAMENTO
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "tipoConta")]
    pub tipo_conta: Option<SantanderAccountType>,
}

/// Represents calendar information for recurring charge (cobr endpoint)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderPixAutomaticoCobrCalendario {
    /// Due date in YYYY-MM-DD format
    pub data_de_vencimento: String,
}

/// Represents value information for recurring charge (cobr endpoint)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderPixAutomaticoCobrValor {
    /// The amount to be charged
    pub original: StringMajorUnit,
}

/// Request for creating a recurring Pix Automático charge (cobr endpoint)
/// This is used when a MIT (Merchant Initiated Transaction) mandate is active
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderPixAutomaticoCobrRequest {
    /// Recurring charge ID - must be 29 alphanumeric characters (RR + 27 chars)
    pub id_rec: Secret<String>,
    /// Additional information about the charge
    #[serde(skip_serializing_if = "Option::is_none")]
    pub info_adicional: Option<String>,
    /// Calendar information (due date)
    pub calendario: SantanderPixAutomaticoCobrCalendario,
    /// Amount to be charged
    pub valor: SantanderPixAutomaticoCobrValor,
    /// Whether to adjust due date to next business day if due date falls on non-business day
    pub ajuste_dia_util: bool,
    /// Receiver/beneficiary details
    pub recebedor: SantanderPixAutomaticoRecebedor,
    /// Optional debtor information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub devedor: Option<SantanderDebtor>,
}

impl hyperswitch_masking::SerializableSecret for SantanderAccountType {}

/// Portuguese equivalents for account types used in Santander API
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum SantanderAccountType {
    /// Checking account
    #[serde(rename = "CORRENTE")]
    Corrente,
    /// Savings account
    #[serde(rename = "POUPANCA")]
    Poupanca,
    /// Payment account
    #[serde(rename = "PAGAMENTO")]
    Pagamento,
}

pub enum AccessTokenUrlPath {
    Leg1,
    Leg2,
    Boleto,
}

use common_utils::types::{FloatMajorUnit, StringMajorUnit};
use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::connectors::santander::requests;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Payer {
    pub name: Secret<String>,
    pub document_type: common_enums::DocumentKind,
    pub document_number: Option<Secret<String>>,
    pub address: Secret<String>,
    pub neighborhood: Secret<String>,
    pub city: String,
    pub state: Secret<String>,
    pub zipcode: Secret<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Beneficiary {
    pub name: Option<Secret<String>>,
    pub document_type: Option<common_enums::DocumentKind>,
    pub document_number: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BoletoDocumentKind {
    #[serde(rename = "DUPLICATA_MERCANTIL")]
    DuplicateMercantil,
    #[serde(rename = "DUPLICATA_SERVICO")]
    DuplicateService,
    #[serde(rename = "NOTA_PROMISSORIA")]
    PromissoryNote,
    #[serde(rename = "NOTA_PROMISSORIA_RURAL")]
    RuralPromissoryNote,
    #[serde(rename = "RECIBO")]
    Receipt,
    #[serde(rename = "APOLICE_SEGURO")]
    InsurancePolicy,
    #[serde(rename = "BOLETO_CARTAO_CREDITO")]
    BillCreditCard,
    #[serde(rename = "BOLETO_PROPOSTA")]
    BillProposal,
    #[serde(rename = "BOLETO_DEPOSITO_APORTE")]
    BoletoDepositoAponte,
    #[serde(rename = "CHEQUE")]
    Check,
    #[serde(rename = "NOTA_PROMISSORIA_DIRETA")]
    DirectPromissoryNote,
    #[serde(rename = "OUTROS")]
    Others,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SantanderAdditionalInfo {
    #[serde(rename = "nome")]
    pub name: String,
    #[serde(rename = "valor")]
    pub value: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum SantanderPaymentStatus {
    #[serde(rename = "ATIVA")]
    Active,
    #[serde(rename = "CONCLUIDA")]
    Completed,
    #[serde(rename = "REMOVIDA_PELO_USUARIO_RECEBEDOR")]
    RemovedByReceivingUser,
    #[serde(rename = "REMOVIDA_PELO_PSP")]
    RemovedByPSP,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SantanderVoidStatus {
    #[serde(rename = "REMOVIDA_PELO_USUARIO_RECEBEDOR")]
    RemovedByReceivingUser,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SantanderPaymentsResponse {
    PixQRCode(Box<SantanderPixQRCodePaymentsResponse>),
    Boleto(Box<SantanderBoletoPaymentsResponse>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SantanderPixQRCodePaymentsResponse {
    pub status: SantanderPaymentStatus,
    #[serde(rename = "calendario")]
    pub calendar: SantanderCalendarResponse,
    #[serde(rename = "txid")]
    pub transaction_id: String,
    #[serde(rename = "revisao")]
    pub revision: Option<serde_json::Value>,
    #[serde(rename = "devedor")]
    pub debtor: Option<requests::SantanderDebtor>,
    pub location: Option<String>,
    #[serde(rename = "recebedor")]
    pub recipient: Option<Recipient>,
    #[serde(rename = "valor")]
    pub value: requests::SantanderValue,
    #[serde(rename = "chave")]
    pub key: Secret<String>,
    #[serde(rename = "solicitacaoPagador")]
    pub request_payer: Option<String>,
    #[serde(rename = "infoAdicionais")]
    pub additional_info: Option<Vec<SantanderAdditionalInfo>>,
    pub pix: Option<Vec<SantanderPix>>,
    #[serde(rename = "pixCopiaECola")]
    pub pix_qr_code_data: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SantanderBoletoPaymentsResponse {
    pub environment: requests::Environment,
    pub nsu_code: String,
    pub nsu_date: String,
    pub covenant_code: String,
    pub bank_number: Secret<String>,
    pub client_number: Option<common_utils::id_type::CustomerId>,
    pub due_date: String,
    pub issue_date: String,
    pub participant_code: Option<String>,
    pub nominal_value: StringMajorUnit,
    pub payer: Payer,
    pub beneficiary: Option<Beneficiary>,
    pub document_kind: BoletoDocumentKind,
    pub discount: Option<requests::Discount>,
    pub fine_percentage: Option<String>,
    pub fine_quantity_days: Option<String>,
    pub interest_percentage: Option<String>,
    pub deduction_value: Option<FloatMajorUnit>,
    pub protest_type: Option<requests::ProtestType>,
    pub protest_quantity_days: Option<i64>,
    pub write_off_quantity_days: Option<String>,
    pub payment_type: PaymentType,
    pub parcels_quantity: Option<i64>,
    pub value_type: Option<String>,
    pub min_value_or_percentage: Option<f64>,
    pub max_value_or_percentage: Option<f64>,
    pub iof_percentage: Option<f64>,
    pub sharing: Option<Sharing>,
    pub key: Option<Key>,
    pub tx_id: Option<String>,
    pub messages: Option<Vec<String>>,
    pub barcode: Option<String>,
    pub digitable_line: Option<Secret<String>>,
    pub entry_date: Option<String>,
    pub qr_code_pix: Option<String>,
    pub qr_code_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SantanderPixQRCodeSyncResponse {
    pub status: SantanderPaymentStatus,
    #[serde(rename = "calendario")]
    pub calendar: SantanderCalendarResponse,
    #[serde(rename = "txid")]
    pub transaction_id: String,
    #[serde(rename = "revisao")]
    pub revision: serde_json::Value,
    #[serde(rename = "devedor")]
    pub debtor: Option<requests::SantanderDebtor>,
    pub location: Option<String>,
    #[serde(rename = "valor")]
    pub value: requests::SantanderValue,
    #[serde(rename = "chave")]
    pub key: Secret<String>,
    #[serde(rename = "solicitacaoPagador")]
    pub request_payer: Option<String>,
    #[serde(rename = "infoAdicionais")]
    pub additional_info: Option<Vec<SantanderAdditionalInfo>>,
    pub pix: Option<Vec<SantanderPix>>,
    #[serde(rename = "pixCopiaECola")]
    pub pix_qr_code_data: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderPixVoidResponse {
    #[serde(rename = "calendario")]
    pub calendar: SantanderCalendarResponse,
    #[serde(rename = "txid")]
    pub transaction_id: String,
    #[serde(rename = "revisao")]
    pub revision: serde_json::Value,
    #[serde(rename = "devedor")]
    pub debtor: Option<requests::SantanderDebtor>,
    #[serde(rename = "recebedor")]
    pub recebedor: Recipient,
    pub status: SantanderPaymentStatus,
    #[serde(rename = "valor")]
    pub value: ValueResponse,
    #[serde(rename = "pixCopiaECola")]
    pub pix_qr_code_data: Option<Secret<String>>,
    #[serde(rename = "chave")]
    pub key: Secret<String>,
    #[serde(rename = "solicitacaoPagador")]
    pub request_payer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "infoAdicionais")]
    pub additional_info: Option<Vec<SantanderAdditionalInfo>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValueResponse {
    #[serde(rename = "original")]
    pub original: String,
    #[serde(rename = "multa")]
    pub fine: Fine,
    #[serde(rename = "juros")]
    pub interest: Interest,
    #[serde(rename = "desconto")]
    pub discount: DiscountResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fine {
    #[serde(rename = "modalidade")]
    pub r#type: String,
    #[serde(rename = "valorPerc")]
    pub perc_value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Interest {
    #[serde(rename = "modalidade")]
    pub r#type: String,
    #[serde(rename = "valorPerc")]
    pub perc_value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscountResponse {
    #[serde(rename = "modalidade")]
    pub r#type: String,
    #[serde(rename = "descontoDataFixa")]
    pub fixed_date_discount: Vec<FixedDateDiscount>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixedDateDiscount {
    #[serde(rename = "data")]
    pub date: String,
    #[serde(rename = "valorPerc")]
    pub perc_value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recipient {
    pub cnpj: Option<Secret<String>>,
    #[serde(rename = "nome")]
    pub name: Option<Secret<String>>,
    #[serde(rename = "nomeFantasia")]
    pub business_name: Option<Secret<String>>,
    #[serde(rename = "logradouro")]
    pub street: Option<Secret<String>>,
    #[serde(rename = "cidade")]
    pub city: Option<Secret<String>>,
    #[serde(rename = "uf")]
    pub state: Option<Secret<String>>,
    #[serde(rename = "cep")]
    pub zip_code: Option<Secret<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SantanderCalendarResponse {
    #[serde(rename = "criacao")]
    pub creation: String,
    #[serde(rename = "expiracao")]
    pub expiration: Option<String>,
    #[serde(rename = "dataDeVencimento")]
    pub due_date: Option<String>,
    #[serde(rename = "validadeAposVencimento")]
    pub validity_after_due: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SantanderPaymentsSyncResponse {
    PixQRCode(Box<SantanderPixQRCodeSyncResponse>),
    Boleto(Box<SantanderBoletoPSyncResponse>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SantanderBoletoPSyncResponse {
    pub link: Option<url::Url>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SantanderPix {
    pub end_to_end_id: Secret<String>,
    #[serde(rename = "txid")]
    pub transaction_id: Secret<String>,
    #[serde(rename = "valor")]
    pub value: String,
    #[serde(rename = "horario")]
    pub time: String,
    #[serde(rename = "infoPagador")]
    pub info_payer: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderUpdateBoletoResponse {
    pub covenant_code: Option<String>,
    pub bank_number: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SantanderRefundStatus {
    InProcessing,
    Returned,
    NotDone,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderRefundResponse {
    pub id: Secret<String>,
    pub rtr_id: Secret<String>,
    #[serde(rename = "valor")]
    pub value: StringMajorUnit,
    #[serde(rename = "horario")]
    pub time: SantanderTime,
    pub status: SantanderRefundStatus,
    #[serde(rename = "motivo")]
    pub reason: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderTime {
    #[serde(rename = "solicitacao")]
    pub request: Option<String>,
    #[serde(rename = "liquidacao")]
    pub liquidation: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SantanderErrorResponse {
    PixQrCode(SantanderPixQRCodeErrorResponse),
    Boleto(SantanderBoletoErrorResponse),
    Generic(SantanderGenericErrorResponse),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SantanderGenericErrorResponse {
    Pattern1(SantanderPattern1ErrorResponse),
    Pattern2(SantanderPattern2ErrorResponse),
    Pattern3(SantanderPattern3ErrorResponse),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SantanderPattern3ErrorResponse {
    pub fault: FaultError,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaultError {
    #[serde(rename = "faultstring")]
    pub fault_string: String,
    pub detail: DetailError,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetailError {
    #[serde(rename = "errorcode")]
    pub error_code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SantanderBoletoErrorResponse {
    #[serde(rename = "_errorCode")]
    pub error_code: i64,
    #[serde(rename = "_message")]
    pub error_message: String,
    #[serde(rename = "_details")]
    pub issuer_error_message: String,
    #[serde(rename = "_timestamp")]
    pub timestamp: String,
    #[serde(rename = "_traceId")]
    pub trace_id: String,
    #[serde(rename = "_errors")]
    pub errors: Option<Vec<ErrorObject>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SantanderPattern1ErrorResponse {
    #[serde(rename = "type")]
    pub card_type: String,
    pub title: String,
    pub status: serde_json::Value,
    pub detail: Option<String>,
    #[serde(rename = "correlationId")]
    pub correlation_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderPattern2ErrorResponse {
    pub timestamp: String,
    pub http_status: String,
    pub details: Option<String>,
    pub error_code: Option<serde_json::Value>,
    pub tracking_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorObject {
    #[serde(rename = "_code")]
    pub code: Option<i64>,
    #[serde(rename = "_field")]
    pub field: Option<String>,
    #[serde(rename = "_message")]
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderPixQRCodeErrorResponse {
    #[serde(rename = "type")]
    pub field_type: Secret<String>,
    pub title: String,
    pub status: String,
    pub detail: Option<String>,
    pub correlation_id: Option<String>,
    #[serde(rename = "violacoes")]
    pub violations: Option<Vec<SantanderViolations>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SantanderViolations {
    #[serde(rename = "razao")]
    pub reason: Option<String>,
    #[serde(rename = "propriedade")]
    pub property: Option<String>,
    #[serde(rename = "valor")]
    pub value: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderWebhookBody {
    pub message: MessageCode,   // meaning of this enum variant is not clear
    pub function: FunctionType, // event type of the webhook
    pub payment_type: WebhookPaymentType,
    pub issue_date: String,
    pub payment_date: String,
    pub bank_code: String,
    pub payment_channel: PaymentChannel,
    pub payment_kind: PaymentKind,
    pub covenant: String,
    pub type_of_person_agreement: common_enums::DocumentKind,
    pub agreement_document: String,
    pub bank_number: String,
    pub client_number: common_utils::id_type::CustomerId,
    pub participant_code: String,
    pub tx_id: String,
    pub payer_document_type: common_enums::DocumentKind,
    pub payer_document_number: String,
    pub payer_name: String,
    pub final_beneficiary_document_type: common_enums::DocumentKind,
    pub final_beneficiary_document_number: String,
    pub final_beneficiary_name: String,
    pub due_date: String,
    pub nominal_value: StringMajorUnit,
    #[serde(rename = "payed_value")]
    pub paid_value: String,
    pub interest_value: String,
    pub fine: String,
    pub deduction_value: String,
    pub rebate_value: String,
    pub iof_value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MessageCode {
    Wbhkpagest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FunctionType {
    Pagamento, // Payment
    Estorno,   // Refund
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WebhookPaymentType {
    Santander,
    OutrosBancos,
    Pix,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
/// Represents the channel through which a boleto payment was made.
pub enum PaymentChannel {
    /// Payment made at a bank branch or ATM (self-service).
    #[serde(rename = "AgenciasAutoAtendimento")]
    BankBranchOrAtm,
    /// Payment made through online banking.
    #[serde(rename = "InternetBanking")]
    OnlineBanking,
    /// Payment made at a physical correspondent agent (e.g., convenience stores, partner outlets).
    #[serde(rename = "CorrespondenteBancarioFisico")]
    PhysicalCorrespondentAgent,
    /// Payment made via Santander’s call center.
    #[serde(rename = "CentralDeAtendimento")]
    CallCenter,
    /// Payment made via electronic file, typically for bulk company payments.
    #[serde(rename = "ArquivoEletronico")]
    ElectronicFile,
    /// Payment made via DDA (Débito Direto Autorizado) / electronic bill presentment system.
    #[serde(rename = "Dda")]
    DirectDebitAuthorized,
    /// Payment made via digital correspondent channels (apps, kiosks, digital partners).
    #[serde(rename = "CorrespondenteBancarioDigital")]
    DigitalCorrespondentAgent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
/// Represents the type of payment instrument used to pay a boleto.
pub enum PaymentKind {
    /// Payment made in cash or physical form (not via account or card).
    Especie,
    /// Payment made via direct debit from a bank account.
    DebitoEmConta,
    /// Payment made via credit card.
    CartaoDeCredito,
    /// Payment made via check.
    Cheque,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum PaymentType {
    #[serde(rename = "REGISTRO")]
    Registration,
    #[serde(rename = "DIVERGENTE")]
    Divergent,
    #[serde(rename = "PARCIAL")]
    Partial,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sharing {
    pub code: String,
    pub value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Key {
    #[serde(rename = "type")]
    pub key_type: Option<String>,
    pub dict_key: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SanatanderAccessTokenResponse {
    Response(SanatanderTokenResponse),
    Error(SantanderTokenErrorResponse),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SanatanderTokenResponse {
    Pix(SanatanderPixAccessTokenResponse),
    Boleto(SanatanderBoletoAccessTokenResponse),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SanatanderPixAccessTokenResponse {
    #[serde(rename = "refreshUrl")]
    pub refresh_url: String,
    pub token_type: String,
    pub client_id: Secret<String>,
    pub access_token: Secret<String>,
    pub scopes: String,
    pub expires_in: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SanatanderBoletoAccessTokenResponse {
    pub access_token: Secret<String>,
    pub expires_in: i64,
    pub token_type: String,
    #[serde(rename = "not-before-policy")]
    pub not_before_policy: i64,
    pub session_state: String,
    pub scope: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SantanderTokenErrorResponse {
    #[serde(rename = "type")]
    pub error_type: String,
    pub title: String,
    pub status: u16,
    pub detail: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QrDataUrlSantander {
    pub qr_code_url: url::Url,
    pub display_to_timestamp: Option<i64>,
    pub variant: Option<api_models::payments::SantanderVariant>,
}

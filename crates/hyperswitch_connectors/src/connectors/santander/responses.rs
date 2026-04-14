use api_models::payments::PollConfig;
use common_utils::types::StringMajorUnit;
use hyperswitch_masking::Secret;
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::connectors::santander::requests;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Payer {
    pub name: Secret<String>,
    pub document_type: SantanderDocumentKind,
    pub document_number: Option<Secret<String>>,
    pub address: Option<Secret<String>>,
    pub neighborhood: Option<Secret<String>>,
    pub city: Option<Secret<String>>,
    pub state: Option<Secret<String>>,
    pub zip_code: Option<Secret<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum SantanderDocumentKind {
    Cnpj,
    Cpf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Beneficiary {
    pub name: Option<Secret<String>>,
    pub document_type: Option<SantanderDocumentKind>,
    pub document_number: Option<Secret<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SantanderBoletoDocumentKind {
    // Used when selling goods/products (commercial invoice).
    DuplicataMercantil,
    // Used when selling services (service invoice).
    DuplicataServico,
    // A standard promissory note — customer promises to pay later.
    NotaPromissoria,
    // Promissory note related to rural/agricultural operations.
    NotaPromissoriaRural,
    // A receipt, usually when the boleto is tied to a receipt-type transaction.
    Recibo,
    // Related to insurance policy payments.
    ApoliceSeguro,
    // Used when the boleto is tied to credit card operations (e.g., card invoice).
    BoletoCartaoCredito,
    // For payments related to commercial proposals/quotes.
    BoletoProposta,
    // For deposit or funding (aporte) into an account (e.g., prepaid wallet top-up).
    BoletoDepositoAporte,
    // Payment related to a cheque transaction.
    Cheque,
    // A direct promissory note (often between borrower and lender directly).
    NotaPromissoriaDireta,
    // Anything that doesn't fit the above categories
    Outros,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SantanderAdditionalInfo {
    // Name
    pub nome: Secret<String>,
    // Value
    pub valor: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SantanderPaymentStatus {
    // Active
    Ativa,
    // Completed
    Concluida,
    // Removed By Receiving User
    RemovidaPeloUsuarioRecebedor,
    // Removed By Psp
    RemovidaPeloPsp,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SantanderVoidStatus {
    // RemovedByReceivingUser
    RemovidaPeloUsuarioRecebedor,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SantanderPaymentsResponse {
    PixQRCode(Box<SantanderPixQRCodePaymentsResponse>),
    Boleto(Box<SantanderBoletoPaymentsResponse>),
    PixAutomaticoCobr(Box<SantanderPixAutomaticoCobrResponse>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SantanderCreatePixPayloadLocationResponse {
    pub id: i64,
    pub location: String,
    pub criacao: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SantanderPixQRCodePaymentsResponse {
    pub status: SantanderPaymentStatus,
    // Calendar
    pub calendario: SantanderCalendarResponse,
    // Transaction Id
    pub txid: String,
    // revision
    pub revisao: serde_json::Value,
    // Debtor
    pub devedor: requests::SantanderDebtor,
    pub location: Option<String>,
    // Recipient
    pub recebedor: Option<Recipient>,
    // Value
    pub valor: requests::SantanderValue,
    // Key
    pub chave: Secret<String>,
    // Request Payer
    pub solicitacao_pagador: Option<String>,
    // Additional Info
    pub info_adicionais: Option<Vec<SantanderAdditionalInfo>>,
    pub pix: Option<Vec<SantanderPix>>,
    // pix_qr_code_data
    pub pix_copia_e_cola: Option<String>,
}

/// Response for Pix Automático recurring charge (cobr endpoint)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderPixAutomaticoCobrResponse {
    /// Recurring charge ID
    pub id_rec: Secret<String>,
    /// Transaction ID
    pub txid: String,
    /// Additional information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub info_adicional: Option<String>,
    /// Calendar information (due date)
    pub calendario: SantanderPixAutomaticoCobrCalendarResponse,
    /// Amount information
    pub valor: requests::SantanderPixAutomaticoCobrValor,
    /// Status of the recurring charge
    pub status: SantanderPixAutomaticoCobrStatus,
    /// Retry policy
    pub politica_retentativa: String,
    /// Adjustment for business days
    pub ajuste_dia_util: bool,
    /// Receiver details
    pub recebedor: requests::SantanderPixAutomaticoRecebedor,
    /// Optional debtor information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub devedor: Option<requests::SantanderDebtor>,
}

/// Calendar information in cobr response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderPixAutomaticoCobrCalendarResponse {
    /// Due date in YYYY-MM-DD format
    pub data_de_vencimento: String,
    /// Creation date in YYYY-MM-DD format
    pub criacao: String,
}

/// Status of Pix Automático recurring charge
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SantanderPixAutomaticoCobrStatus {
    /// Created
    Criada,
    /// Active
    Ativa,
    /// Completed
    Concluida,
    /// Expired
    Expirada,
    /// Rejected
    Rejeitada,
    /// Cancelled
    Cancelada,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderBoletoPaymentsResponse {
    pub environment: requests::Environment,
    pub nsu_code: String,
    pub nsu_date: String,
    pub covenant_code: Secret<String>,
    pub bank_number: String,
    pub client_number: Option<String>,
    #[serde(with = "common_utils::custom_serde::date_only")]
    pub due_date: PrimitiveDateTime,
    pub issue_date: String,
    pub participant_code: Option<String>,
    pub nominal_value: StringMajorUnit,
    pub payer: Payer,
    pub beneficiary: Option<Beneficiary>,
    pub document_kind: SantanderBoletoDocumentKind,
    pub discount: Option<requests::Discount>,
    pub fine_percentage: Option<String>,
    pub fine_quantity_days: Option<String>,
    pub interest_percentage: Option<String>,
    pub deduction_value: Option<String>,
    pub protest_type: Option<requests::SantanderProtestType>,
    pub protest_quantity_days: Option<String>,
    pub write_off_quantity_days: Option<String>,
    pub payment_type: SantanderBoletoPaymentType,
    pub parcels_quantity: Option<String>,
    pub value_type: Option<String>,
    pub min_value_or_percentage: Option<String>,
    pub max_value_or_percentage: Option<String>,
    pub iof_percentage: Option<String>,
    pub sharing: Option<Vec<Sharing>>,
    pub key: Option<Key>,
    pub tx_id: Option<String>,
    pub messages: Option<Vec<String>>,
    pub bar_code: Option<Secret<String>>,
    pub digitable_line: Option<Secret<String>>,
    pub entry_date: Option<String>,
    pub qr_code_pix: Option<String>,
    pub qr_code_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderPixQRCodeSyncResponse {
    pub status: SantanderPaymentStatus,
    // Calendar
    pub calendario: SantanderCalendarResponse,
    // Transaction Id
    pub txid: String,
    // Revision
    pub revisao: Option<serde_json::Value>,
    // Debtor
    pub devedor: Option<requests::SantanderDebtor>,
    pub location: Option<String>,
    // Value
    pub valor: requests::SantanderValue,
    // Key
    pub chave: Secret<String>,
    // Request Payer
    pub solicitacao_pagador: Option<String>,
    // Additional Info
    pub info_adicionais: Option<Vec<SantanderAdditionalInfo>>,
    pub pix: Option<Vec<SantanderPix>>,
    // Pix QR Code Data
    pub pix_copia_e_cola: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderPixVoidResponse {
    // Calendar
    pub calendario: SantanderCalendarResponse,
    // Transaction Id
    pub txid: String,
    // Revision
    pub revisao: Option<serde_json::Value>,
    // Debtor
    pub devedor: Option<requests::SantanderDebtor>,
    // Recipient
    pub recebedor: Recipient,
    // Status
    pub status: SantanderPaymentStatus,
    // Value
    pub valor: ValueResponse,
    // Pix QR Code Data
    pub pix_copia_e_cola: Option<Secret<String>>,
    // Key
    pub chave: Secret<String>,
    // Request Payer
    pub solicitacao_pagador: Option<String>,
    // Additional Info
    pub info_adicionais: Option<Vec<SantanderAdditionalInfo>>,
    pub pix: Option<Vec<SantanderPix>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SantanderVoidResponse {
    Pix(Box<SantanderPixVoidResponse>),
    Boleto(Box<SantanderBoletoVoidResponse>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderBoletoVoidResponse {
    pub covenant_code: String,
    pub bank_number: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValueResponse {
    // Original payment amount
    pub original: String,
    // Fine (penalty) details
    pub multa: Option<Fine>,
    // Interest details
    pub juros: Option<Interest>,
    // Discount details
    pub desconto: Option<DiscountResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Fine {
    // Type or mode of fine (e.g., percentage or fixed amount)
    pub modalidade: String,
    // Fine value or percentage applied
    pub valor_perc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Interest {
    // Type or mode of interest (e.g., daily rate or fixed)
    pub modalidade: String,
    // Interest value or percentage applied
    pub valor_perc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscountResponse {
    // Type or mode of discount (e.g., fixed date or percentage)
    pub modalidade: String,
    // List of discounts applicable on specific fixed dates
    pub desconto_data_fixa: Vec<FixedDateDiscount>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FixedDateDiscount {
    // Date on which the discount is valid
    pub data: String,
    // Discount value or percentage applied
    pub valor_perc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Recipient {
    // Recipient’s CNPJ (business tax ID)
    pub cnpj: Option<Secret<String>>,
    // Recipient’s legal name
    pub nome: Option<Secret<String>>,
    // Recipient’s business or trade name
    pub nome_fantasia: Option<Secret<String>>,
    // Street address of the recipient
    pub logradouro: Option<Secret<String>>,
    // City where the recipient is located
    pub cidade: Option<Secret<String>>,
    // State (federal unit) of the recipient
    pub uf: Option<Secret<String>>,
    // Postal code (ZIP code) of the recipient
    pub cep: Option<Secret<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SantanderCalendarResponse {
    // Date and time when the payment was created
    pub criacao: String,
    // Expiration time of the payment (if applicable)
    pub expiracao: Option<String>,
    // Due date for the payment
    pub data_de_vencimento: Option<String>,
    // Validity period after the due date
    pub validade_apos_vencimento: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SantanderPaymentsSyncResponse {
    PixQRCode(Box<SantanderPixQRCodeSyncResponse>),
    Boleto(Box<SantanderBoletoPSyncResponse>),
    PixAutomaticoConsultAndActivateJourney(Box<SantanderPixAutomaticRecResponse>),
    PixAutomaticoCobrSync(Box<SantanderPixAutomaticoCobrSyncResponse>),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SantanderPix {
    // Unique PIX transaction identifier
    pub end_to_end_id: Secret<String>,
    // Transaction ID associated with the payment
    pub txid: Secret<String>,
    // Transaction amount
    pub valor: String,
    // Timestamp when the transaction occurred
    pub horario: String,
    // Optional information provided by the payer
    pub info_pagador: Option<String>,
}

/// Response for consulting a recurring charge via cobr endpoint (GET /api/v1/cobr/{txid})
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderPixAutomaticoCobrSyncResponse {
    /// Recurring charge ID (idRec)
    pub id_rec: Secret<String>,
    /// Transaction ID
    pub txid: String,
    /// Additional information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub info_adicional: Option<String>,
    /// Calendar information (due date, creation date)
    pub calendario: SantanderPixAutomaticoCobrCalendarResponse,
    /// Amount information
    pub valor: requests::SantanderPixAutomaticoCobrValor,
    /// Adjustment for business days
    pub ajuste_dia_util: bool,
    /// Receiver details
    pub recebedor: serde_json::Value,
    /// Status of the recurring charge
    pub status: SantanderPixAutomaticoCobrStatus,
    /// Retry policy
    pub politica_retentativa: String,
    /// Optional debtor information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub devedor: Option<serde_json::Value>,
    /// Pix transaction details (present when payment is completed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pix: Option<Vec<SantanderCobrSyncPix>>,
    /// Status update history
    #[serde(skip_serializing_if = "Option::is_none")]
    pub atualizacao: Option<Vec<SantanderCobrSyncStatusUpdate>>,
    /// Attempt history
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tentativas: Option<Vec<SantanderCobrSyncTentativa>>,
}

/// Pix transaction detail inside cobr sync response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderCobrSyncPix {
    /// EndToEndIdentification from PACS messages
    pub end_to_end_id: Secret<String>,
    /// Transaction ID
    pub txid: String,
    /// Value information
    pub valor: serde_json::Value,
    /// Timestamp when the Pix was processed
    pub horario: String,
    /// Optional payer info
    #[serde(skip_serializing_if = "Option::is_none")]
    pub info_pagador: Option<String>,
}

/// Status update entry in cobr sync response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderCobrSyncStatusUpdate {
    /// Status of the charge
    pub status: String,
    /// Date/time of the status update (RFC 3339)
    pub data: String,
}

/// Attempt history entry in cobr sync response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderCobrSyncTentativa {
    /// Liquidation date (YYYY-MM-DD)
    pub data_liquidacao: Option<String>,
    /// Attempt type (AGND, NTAG, RIFL)
    pub tipo: String,
    /// Attempt status
    pub status: String,
    /// EndToEndIdentification
    pub end_to_end_id: Option<Secret<String>>,
    /// Status update history for this attempt
    #[serde(skip_serializing_if = "Option::is_none")]
    pub atualizacao: Option<Vec<SantanderCobrSyncStatusUpdate>>,
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
    /// The refund is currently being processed and not yet completed
    EmProcessamento,
    /// The refund has been successfully completed and the amount was returned
    Devolvido,
    /// The refund was not carried out
    NaoRealizado,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderRefundResponse {
    // Hyperswitch Refund Id
    pub id: Secret<String>,
    // Connector Refund Id
    pub rtr_id: Secret<String>, // Need to confirm with Santander on what this id is
    // Refund amount
    pub valor: StringMajorUnit,
    // Time information related to the refund
    pub horario: SantanderTime,
    // Refund status
    pub status: SantanderRefundStatus,
    // Reason for the refund
    pub motivo: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderTime {
    // Time when the refund was requested
    pub solicitacao: Option<String>,
    // Time when the refund was completed (settled)
    pub liquidacao: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SantanderPixAutomaticoErrorResponse {
    pub code: i32,
    pub message: String,
    pub level: String,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SantanderErrorResponse {
    PixQrCode(SantanderPixQRCodeErrorResponse),
    Boleto(SantanderBoletoErrorResponse),
    // Pix Automatico API returns 401 Unauthorized when access token is expired
    PixAutomatico(SantanderPixAutomaticoErrorResponse),
    Generic(SantanderGenericErrorResponse),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SantanderGenericErrorResponse {
    Pattern1(SantanderPattern1ErrorResponse),
    // Validation Errors or when wrong access token is passed
    Pattern2(SantanderPattern2ErrorResponse),
    // When JWT is expired
    Pattern3(SantanderPattern3ErrorResponse),
    // On Refund Failures
    Pattern4(SantanderPattern4ErrorResponse),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SantanderPattern4ErrorResponse {
    pub timestamp: Option<String>,
    #[serde(rename = "httpStatusCode")]
    pub http_status_code: Option<String>,
    pub detail: Option<String>,
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
    pub issuer_error_message: Option<String>,
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
    pub code: Option<String>,
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
    // could be i64/String as well
    pub status: serde_json::Value,
    pub detail: Option<String>,
    pub correlation_id: Option<String>,
    // Violations - required to distinguish from Pattern1ErrorResponse
    pub violacoes: Vec<SantanderViolations>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderViolations {
    // Description or reason for the violation
    pub razao: Option<String>,
    // Name of the property or field that caused the violation
    pub propriedade: Option<String>,
    // Value associated with the violation
    pub valor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
/// Represents the channel through which a boleto payment was made.
pub enum PaymentChannel {
    /// Payment made at a bank branch or ATM (self-service).
    AgenciasAutoAtendimento,
    /// Payment made through online banking.
    InternetBanking,
    /// Payment made at a physical correspondent agent (e.g., convenience stores, partner outlets).
    CorrespondenteBancarioFisico,
    /// Payment made via Santander’s call center.
    CentralDeAtendimento,
    /// Payment made via electronic file, typically for bulk company payments.
    ArquivoEletronico,
    /// Payment made via DDA (Débito Direto Autorizado) / electronic bill presentment system.
    Dda,
    /// Payment made via digital correspondent channels (apps, kiosks, digital partners).
    CorrespondenteBancarioDigital,
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
/// Represents the type of boleto payment or registration action.
pub enum SantanderBoletoPaymentType {
    /// Only the exact nominal value can be paid
    Registro,
    /// Payer can pay any amount within a range, min to max value
    Divergente,
    /// Payer can make up to 99 partial payments
    Parcial,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sharing {
    // A 2-digit identifier for a split rule
    pub code: String,
    // Exact monetary amount assigned to that split
    pub value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Key {
    #[serde(rename = "type")]
    pub key_type: Option<SantanderPixKeyType>,
    pub dict_key: Option<Secret<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum SantanderPixKeyType {
    Cpf,
    Cnpj,
    Email,
    Cellular,
    Evp,
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
    PixAutomaticoBoleto(SantanderPixAutomaticoOrBoletoAccessTokenResponse),
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
pub struct SantanderPixAutomaticoOrBoletoAccessTokenResponse {
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
    pub variant: Option<common_enums::enums::ExpiryType>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SantanderUpdateMetadataResponse {
    Pix(Box<SantanderPixQRCodePaymentsResponse>),
    Boleto(Box<SantanderUpdateBoletoResponse>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NsuComposite {
    pub nsu_code: String,
    pub nsu_date: String,
    pub environment: String,
    pub covenant_code: String,
    pub bank_number: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderBoletoPSyncResponse {
    #[serde(rename = "_content")]
    pub content: Vec<SantanderBoletoContent>,
    #[serde(rename = "_pageable")]
    pub pageable: SantanderPaginationMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderPaginationMetadata {
    #[serde(rename = "_limit")]
    pub limit: Option<i32>,

    #[serde(rename = "_offset")]
    pub offset: Option<i32>,

    #[serde(rename = "_pageNumber")]
    pub page_number: Option<i32>,

    #[serde(rename = "_pageElements")]
    pub page_elements: Option<i32>,

    #[serde(rename = "_totalPages")]
    pub total_pages: Option<i32>,

    #[serde(rename = "_totalElements")]
    pub total_elements: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderBoletoContent {
    pub nsu_code: Option<String>,
    pub nsu_date: Option<String>,
    pub covenant_code: Secret<String>,
    pub bank_number: String,
    pub client_number: Option<String>,
    pub status: SantanderBoletoStatus,
    pub status_complement: Option<String>,
    pub due_date: String,
    pub issue_date: String,
    pub nominal_value: StringMajorUnit,
    pub payer: Payer,
    pub beneficiary: Beneficiary,
    pub fine_percentage: Option<String>,
    pub fine_quantity_days: Option<String>,
    pub interest_percentage: Option<String>,
    pub discount: Option<requests::Discount>,
    pub deduction_value: Option<String>,
    pub protest_type: Option<String>,
    pub protest_quantity_days: Option<String>,
    pub payment_type: Option<String>,
    pub parcels_quantity: Option<String>,
    pub min_value_or_percentage: Option<String>,
    pub max_value_or_percentage: Option<String>,
    pub iof_percentage: Option<String>,
    pub payment: SantanderPaymentDetails,
    pub barcode: Option<Secret<String>>,
    pub digitable_line: Option<Secret<String>>,
    pub entry_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderPaymentDetails {
    pub paid_value: Option<StringMajorUnit>,
    pub interest_value: Option<StringMajorUnit>,
    pub fine_value: Option<StringMajorUnit>,
    pub deduction_value: Option<StringMajorUnit>,
    pub rebate_value: Option<StringMajorUnit>,
    pub iof_value: Option<StringMajorUnit>,
    pub date: Option<String>,
    #[serde(rename = "type")]
    pub bank_type: Option<String>,
    pub bank_code: Option<String>,
    pub channel: Option<String>,
    pub kind: Option<String>,
    pub credit_date: Option<String>,
    pub tx_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SantanderBoletoStatus {
    /// The boleto is registered and waiting for payment.
    /// It is currently valid and within its expiration period.
    Ativo,
    /// The boleto has been cancelled or removed from the bank's
    Baixado,
    /// The boleto has been paid in full. The funds have been cleared and settled.
    Liquidado,
    /// A partial payment was made
    LiquidadoParcialmente,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderPixAutomaticSolicitationResponse {
    /// Unique identifier for the recurrence solicitation request
    pub id_solic_rec: Secret<String>,
    /// Unique identifier for the recurrence
    pub id_rec: Secret<String>,
    /// Calendar information for the recurrence (expiration, start, end, frequency)
    pub calendario: SantanderPixAutomaticoCalendario,
    /// Recipient/Destination information for the recurrence
    pub destinatario: SantanderPixAutomaticoDestinatario,
    /// Current status of the recurrence solicitation
    pub status: Option<RecurrenceStatus>,
    /// Update history of the recurrence solicitation
    pub atualizacao: Option<Vec<SantanderPixAutomaticoAtualizacao>>,
    /// Complete payload containing detailed recurrence information
    pub rec_payload: Option<SantanderPixAutomaticoRecPayload>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderPixAutomaticoCalendario {
    /// Expiration date of the recurrence solicitation
    pub data_expiracao_solicitacao: Option<String>,
    /// Start date of the recurrence
    pub data_inicial: Option<String>,
    /// End date of the recurrence
    pub data_final: Option<String>,
    /// Periodicity/frequency of the recurrence (e.g., monthly, weekly)
    pub periodicidade: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderPixAutomaticoDestinatario {
    /// Recipient's bank account number
    pub conta: Option<String>,
    /// ISPB code of the recipient's financial institution participant
    pub ispb_participante: Option<String>,
    /// Recipient's bank branch/agency number
    pub agencia: Option<String>,
    /// Recipient's CPF (individual tax ID)
    pub cpf: Option<Secret<String>>,
    /// Recipient's CNPJ (business tax ID)
    pub cnpj: Option<Secret<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderPixAutomaticoAtualizacao {
    /// Status of the recurrence at the time of update
    pub status: Option<RecurrenceStatus>,
    /// Date when the status update occurred
    pub data: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderPixAutomaticoRecPayload {
    /// Unique identifier for the recurrence
    pub id_rec: Secret<String>,
    /// Link/relationship information between debtor and creditor
    pub vinculo: SantanderPixAutomaticoVinculo,
    /// Calendar information for the recurrence
    pub calendario: SantanderPixAutomaticoCalendario,
    /// Value information for the recurrence (recurrence amount and minimum amount)
    pub valor: Option<SantanderPixAutomaticoValor>,
    /// Recipient information for the recurrence
    pub recebedor: SantanderPixAutomaticoRecebedor,
    /// Update history of the recurrence
    pub atualizacao: Vec<SantanderPixAutomaticoAtualizacao>,
    /// Retry policy for failed payments in the recurrence
    pub politica_retentativa: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderPixAutomaticoVinculo {
    /// Object or subject of the recurrence link/agreement
    pub objeto: Option<String>,
    /// Debtor/Payer information in the recurrence relationship
    pub devedor: SantanderPixAutomaticoDevedor,
    /// Contract identifier linking the recurrence to a contract
    pub contrato: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderPixAutomaticoDevedor {
    /// Debtor's CPF (individual tax ID)
    pub cpf: Option<Secret<String>>,
    /// Debtor's CNPJ (business tax ID)
    pub cnpj: Option<Secret<String>>,
    /// Debtor's full name
    pub nome: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderPixAutomaticoRecebedor {
    /// Covenant/agreement identifier for the recipient
    pub convenio: Option<String>,
    /// Recipient's CNPJ (business tax ID)
    pub cnpj: Option<Secret<String>>,
    /// Recipient's full name
    pub nome: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderPixAutomaticRecResponse {
    /// Unique identifier for the recurrence
    pub id_rec: Secret<String>,
    /// Link/relationship information between debtor and creditor
    pub vinculo: SantanderPixAutomaticoVinculo,
    /// Calendar information for the recurrence
    pub calendario: SantanderPixAutomaticoCalendario,
    /// Value information for the recurrence
    pub valor: Option<SantanderPixAutomaticoValor>,
    /// Recipient information for the recurrence
    pub recebedor: SantanderPixAutomaticoRecebedor,
    /// Payer information for the recurrence
    pub pagador: Option<SantanderPixAutomaticoPagador>,
    /// Current status of the recurrence
    pub status: RecurrenceStatus,
    /// Retry policy for failed payments in the recurrence
    pub politica_retentativa: Option<String>,
    /// Location information for the recurrence (QR code location)
    pub loc: Option<SantanderPixAutomaticoLoc>,
    /// Update history of the recurrence
    pub atualizacao: Vec<SantanderPixAutomaticoAtualizacao>,
    /// Closure/termination information if the recurrence was ended
    pub encerramento: Option<SantanderPixAutomaticoEncerramento>,
    /// Associated solicitation requests for the recurrence
    pub solicitacao: Option<Vec<SantanderPixAutomaticSolicitationResponse>>,
    /// Activation information for the recurrence
    pub ativacao: Option<SantanderPixAutomaticoAtivacao>,
    /// QR code data associated with the recurrence
    // #[serde(rename = "dadosQR", alias = "dadosqr")]
    #[serde(rename = "dadosQR")]
    pub dados_qr: Option<SantanderPixAutomaticQrData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderPixAutomaticoPagador {
    /// ISPB code of the payer's financial institution participant
    pub ispb_participante: Option<String>,
    /// Municipality code where the payer is located
    pub cod_mun: Option<String>,
    /// Payer's CPF (individual tax ID)
    pub cpf: Option<Secret<String>>,
    /// Payer's CNPJ (business tax ID)
    pub cnpj: Option<Secret<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderPixAutomaticoLoc {
    /// Unique identifier for the location/QR code
    pub id: Option<i64>,
    /// URL or reference to the QR code location
    pub location: Option<String>,
    /// Date/time when the location was created
    pub criacao: Option<String>,
    /// Recurrence identifier associated with this location
    pub id_rec: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderPixAutomaticoEncerramento {
    /// Rejection information if the recurrence was closed due to rejection
    pub rejeicao: Option<SantanderPixAutomaticoRejeicao>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderPixAutomaticoRejeicao {
    /// Error code for the rejection
    pub codigo: Option<String>,
    /// Description of the rejection reason
    pub descricao: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderPixAutomaticoAtivacao {
    /// Type of journey/flow used to activate the recurrence
    pub tipo_jornada: Option<String>,
    /// Data associated with the activation journey
    pub dados_jornada: Option<SantanderPixAutomaticoDadosJornada>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderPixAutomaticoDadosJornada {
    /// Transaction ID associated with the activation journey
    pub txid: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderPixAutomaticQrData {
    /// Journey type/flow for the QR code
    pub jornada: Option<SantanderJourneyType>,
    /// PIX copy-and-paste string (Copia e Cola) for the QR code
    #[serde(rename = "pixCopiaECola")]
    pub pix_copia_e_cola: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaitScreenData {
    /// Timestamp from which the wait screen should be displayed
    pub display_from_timestamp: i128,
    /// Timestamp until which the wait screen should be displayed (optional)
    pub display_to_timestamp: Option<i128>,
    /// Configuration for polling updates during the wait screen
    pub poll_config: Option<PollConfig>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum SantanderJourneyType {
    // Paying user accepted the recurrence through external notification to the ecosystem
    #[serde(rename = "JORNADA_1")]
    Jornada1,
    // Paying user accepted the recurrence by reading the recurrence QR Code
    #[serde(rename = "JORNADA_2")]
    Jornada2,
    // Paying user initiated the recurrence by reading a composite QR Code + paying an immediate charge
    #[serde(rename = "JORNADA_3")]
    Jornada3,
    // Paying user initiated the recurrence by reading a composite QR Code + paying an scheduled charge
    #[serde(rename = "JORNADA_4")]
    Jornada4,
    // Initial value after creation and before recurrence activation
    #[serde(rename = "AGUARDANDO_DEFINICAO")]
    AguardandoDefinicao,
}

/// Status of the recurring payment mandate
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum RecurrenceStatus {
    /// Created (awaiting approval)
    Criada,
    /// Approved (active)
    Aprovada,
    /// Rejected
    Rejeitada,
    /// Expired
    Expirada,
    /// Cancelled
    Cancelada,
    /// Received
    Recebida,
    // Sent
    Enviada,
    // Accepted
    Aceita,
    /// Unknown status
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderPixAutomaticoValor {
    /// Recurrence amount - the regular payment amount
    pub valor_rec: Option<StringMajorUnit>,
    /// Minimum amount required from the recipient/creditor
    pub valor_minimo_recebedor: Option<StringMajorUnit>,
}

// SetupMandate (Pix Automatico) Response Structures
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderSetupMandateResponse {
    /// Recurrence ID
    pub id_rec: Secret<String>,
    /// Information about the recurring object/service
    pub vinculo: RecurrenceLinkResponse,
    /// Calendar information for the recurrence
    pub calendario: RecurrenceCalendarResponse,
    /// Receiver information
    pub recebedor: RecurrenceReceiver,
    /// Status of the recurrence
    pub status: RecurrenceStatus,
    /// Retry policy for failed payments
    pub politica_retentativa: requests::RetryPolicy,
    /// Location information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub loc: Option<LocationResponse>,
    /// Update history
    pub atualizacao: Vec<RecurrenceStatusUpdate>,
}

/// Information linking the recurrence to the service/contract
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecurrenceLinkResponse {
    /// Description of the recurring object/service
    pub objeto: String,
    /// Debtor information
    pub devedor: RecurrenceDebtorResponse,
    /// Contract identifier
    pub contrato: String,
}

/// Debtor information for recurring payments
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecurrenceDebtorResponse {
    /// CNPJ (Business tax ID)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cnpj: Option<Secret<String>>,
    /// CPF (Individual tax ID)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpf: Option<Secret<String>>,
    /// Name of the debtor
    pub nome: Secret<String>,
}

/// Calendar information for the recurring payment schedule
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecurrenceCalendarResponse {
    /// Initial date in YYYY-MM-DD format
    pub data_inicial: String,
    /// Periodicity of the recurrence
    pub periodicidade: requests::Periodicidade,
    /// Optional end date in YYYY-MM-DD format
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_final: Option<String>,
}

/// Receiver (merchant) information for recurring payments
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecurrenceReceiver {
    /// Receiver's CNPJ (Business tax ID)
    pub cnpj: Secret<String>,
    /// Receiver's name
    pub nome: Secret<String>,
    /// Covenant/agreement code
    pub convenio: String,
}

/// Location information for QR code
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocationResponse {
    /// Location ID
    pub id: i64,
    /// Location URL
    pub location: String,
    /// Creation timestamp
    pub criacao: String,
    /// Recurrence ID
    pub id_rec: Secret<String>,
}

/// Status update history entry for the recurrence
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecurrenceStatusUpdate {
    /// Status of the recurrence
    pub status: Option<RecurrenceStatus>,
    /// Date/time of this status update
    pub data: String,
}

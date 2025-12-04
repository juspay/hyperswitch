use common_utils::types::StringMajorUnit;
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
    pub zip_code: Secret<String>,
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
    pub nome: String,
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
pub enum SantanderVoidStatus {
    // RemovedByReceivingUser
    RemovidaPeloUsuarioRecebedor,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SantanderPaymentsResponse {
    PixQRCode(Box<SantanderPixQRCodePaymentsResponse>),
    Boleto(Box<SantanderBoletoPaymentsResponse>),
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
    pub info_adicionais: Vec<SantanderAdditionalInfo>,
    pub pix: Option<Vec<SantanderPix>>,
    // pix_qr_code_data
    pub pix_copia_e_cola: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderBoletoPaymentsResponse {
    pub environment: requests::Environment,
    pub nsu_code: String,
    pub nsu_date: String,
    pub covenant_code: String,
    pub bank_number: String,
    pub client_number: Option<String>,
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
    pub deduction_value: Option<String>,
    pub protest_type: Option<requests::ProtestType>,
    pub protest_quantity_days: Option<String>,
    pub write_off_quantity_days: Option<String>,
    pub payment_type: PaymentType,
    pub parcels_quantity: Option<String>,
    pub value_type: Option<String>,
    pub min_value_or_percentage: Option<String>,
    pub max_value_or_percentage: Option<String>,
    pub iof_percentage: Option<String>,
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
    pub multa: Fine,
    // Interest details
    pub juros: Interest,
    // Discount details
    pub desconto: DiscountResponse,
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
    pub expiracao: String,
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SantanderBoletoPSyncResponse {
    pub link: Option<url::Url>,
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
    // Unique refund identifier
    pub id: Secret<String>,
    // Unique RTR (Return) identifier
    pub rtr_id: Secret<String>,
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
    // Validation Errors or when wrong access token is passed
    Pattern2(SantanderPattern2ErrorResponse),
    // When JWT is expired
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
    pub status: String,
    pub detail: Option<String>,
    pub correlation_id: Option<String>,
    // Violations
    pub violacoes: Option<Vec<SantanderViolations>>,
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
pub enum PaymentType {
    /// Only the exact nominal value can be paid
    Registro,
    /// Payer can pay any amount within a range, min to max value
    Divergente,
    /// Payer can make up to 99 partial payments
    Parcial,
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
    pub variant: Option<api_models::payments::ExpiryType>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SantanderUpdateMetadataResponse {
    Pix(Box<SantanderPixQRCodePaymentsResponse>),
    Boleto(Box<SantanderUpdateBoletoResponse>),
}

// FIXME: Why were these data types grouped this way?
//
// Folder `types` is strange for Rust ecosystem, nevertheless it might be okay.
// But folder `enum` is even more strange I unlikely okay. Why should not we introduce folders `type`, `structs` and `traits`? :)
// Is it better to split data types according to business logic instead.
// For example, customers/address/dispute/mandate is "models".
// Separation of concerns instead of separation of forms.

pub mod api;
pub mod domain;
pub mod storage;
pub mod transformers;

use std::marker::PhantomData;

pub use api_models::enums::Connector;
use common_utils::{pii, pii::Email};
use error_stack::{IntoReport, ResultExt};
use masking::Secret;

use self::{api::payments, storage::enums as storage_enums};
pub use crate::core::payments::PaymentAddress;
use crate::{core::errors, services};

pub type PaymentsAuthorizeRouterData =
    RouterData<api::Authorize, PaymentsAuthorizeData, PaymentsResponseData>;
pub type PaymentsPreProcessingRouterData =
    RouterData<api::PreProcessing, PaymentsPreProcessingData, PaymentsResponseData>;
pub type PaymentsAuthorizeSessionTokenRouterData =
    RouterData<api::AuthorizeSessionToken, AuthorizeSessionTokenData, PaymentsResponseData>;
pub type PaymentsCompleteAuthorizeRouterData =
    RouterData<api::CompleteAuthorize, CompleteAuthorizeData, PaymentsResponseData>;
pub type PaymentsInitRouterData =
    RouterData<api::InitPayment, PaymentsAuthorizeData, PaymentsResponseData>;
pub type PaymentsSyncRouterData = RouterData<api::PSync, PaymentsSyncData, PaymentsResponseData>;
pub type PaymentsCaptureRouterData =
    RouterData<api::Capture, PaymentsCaptureData, PaymentsResponseData>;
pub type PaymentsCancelRouterData = RouterData<api::Void, PaymentsCancelData, PaymentsResponseData>;
pub type PaymentsSessionRouterData =
    RouterData<api::Session, PaymentsSessionData, PaymentsResponseData>;
pub type RefundsRouterData<F> = RouterData<F, RefundsData, RefundsResponseData>;
pub type RefundExecuteRouterData = RouterData<api::Execute, RefundsData, RefundsResponseData>;
pub type RefundSyncRouterData = RouterData<api::RSync, RefundsData, RefundsResponseData>;
pub type TokenizationRouterData =
    RouterData<api::PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>;
pub type ConnectorCustomerRouterData =
    RouterData<api::CreateConnectorCustomer, ConnectorCustomerData, PaymentsResponseData>;

pub type RefreshTokenRouterData =
    RouterData<api::AccessTokenAuth, AccessTokenRequestData, AccessToken>;

pub type PaymentsResponseRouterData<R> =
    ResponseRouterData<api::Authorize, R, PaymentsAuthorizeData, PaymentsResponseData>;
pub type PaymentsCancelResponseRouterData<R> =
    ResponseRouterData<api::Void, R, PaymentsCancelData, PaymentsResponseData>;
pub type PaymentsSyncResponseRouterData<R> =
    ResponseRouterData<api::PSync, R, PaymentsSyncData, PaymentsResponseData>;
pub type PaymentsSessionResponseRouterData<R> =
    ResponseRouterData<api::Session, R, PaymentsSessionData, PaymentsResponseData>;
pub type PaymentsInitResponseRouterData<R> =
    ResponseRouterData<api::InitPayment, R, PaymentsAuthorizeData, PaymentsResponseData>;
pub type PaymentsCaptureResponseRouterData<R> =
    ResponseRouterData<api::Capture, R, PaymentsCaptureData, PaymentsResponseData>;
pub type TokenizationResponseRouterData<R> = ResponseRouterData<
    api::PaymentMethodToken,
    R,
    PaymentMethodTokenizationData,
    PaymentsResponseData,
>;
pub type ConnectorCustomerResponseRouterData<R> = ResponseRouterData<
    api::CreateConnectorCustomer,
    R,
    ConnectorCustomerData,
    PaymentsResponseData,
>;

pub type RefundsResponseRouterData<F, R> =
    ResponseRouterData<F, R, RefundsData, RefundsResponseData>;

pub type PaymentsAuthorizeType =
    dyn services::ConnectorIntegration<api::Authorize, PaymentsAuthorizeData, PaymentsResponseData>;
pub type PaymentsVerifyType =
    dyn services::ConnectorIntegration<api::Verify, VerifyRequestData, PaymentsResponseData>;
pub type PaymentsPreProcessingType = dyn services::ConnectorIntegration<
    api::PreProcessing,
    PaymentsPreProcessingData,
    PaymentsResponseData,
>;
pub type PaymentsCompleteAuthorizeType = dyn services::ConnectorIntegration<
    api::CompleteAuthorize,
    CompleteAuthorizeData,
    PaymentsResponseData,
>;
pub type PaymentsPreAuthorizeType = dyn services::ConnectorIntegration<
    api::AuthorizeSessionToken,
    AuthorizeSessionTokenData,
    PaymentsResponseData,
>;
pub type PaymentsInitType = dyn services::ConnectorIntegration<
    api::InitPayment,
    PaymentsAuthorizeData,
    PaymentsResponseData,
>;
pub type PaymentsSyncType =
    dyn services::ConnectorIntegration<api::PSync, PaymentsSyncData, PaymentsResponseData>;
pub type PaymentsCaptureType =
    dyn services::ConnectorIntegration<api::Capture, PaymentsCaptureData, PaymentsResponseData>;
pub type PaymentsSessionType =
    dyn services::ConnectorIntegration<api::Session, PaymentsSessionData, PaymentsResponseData>;
pub type PaymentsVoidType =
    dyn services::ConnectorIntegration<api::Void, PaymentsCancelData, PaymentsResponseData>;
pub type TokenizationType = dyn services::ConnectorIntegration<
    api::PaymentMethodToken,
    PaymentMethodTokenizationData,
    PaymentsResponseData,
>;

pub type ConnectorCustomerType = dyn services::ConnectorIntegration<
    api::CreateConnectorCustomer,
    ConnectorCustomerData,
    PaymentsResponseData,
>;

pub type RefundExecuteType =
    dyn services::ConnectorIntegration<api::Execute, RefundsData, RefundsResponseData>;
pub type RefundSyncType =
    dyn services::ConnectorIntegration<api::RSync, RefundsData, RefundsResponseData>;

pub type RefreshTokenType =
    dyn services::ConnectorIntegration<api::AccessTokenAuth, AccessTokenRequestData, AccessToken>;

pub type AcceptDisputeType = dyn services::ConnectorIntegration<
    api::Accept,
    AcceptDisputeRequestData,
    AcceptDisputeResponse,
>;

pub type SubmitEvidenceType = dyn services::ConnectorIntegration<
    api::Evidence,
    SubmitEvidenceRequestData,
    SubmitEvidenceResponse,
>;

pub type UploadFileType =
    dyn services::ConnectorIntegration<api::Upload, UploadFileRequestData, UploadFileResponse>;

pub type RetrieveFileType = dyn services::ConnectorIntegration<
    api::Retrieve,
    RetrieveFileRequestData,
    RetrieveFileResponse,
>;

pub type DefendDisputeType = dyn services::ConnectorIntegration<
    api::Defend,
    DefendDisputeRequestData,
    DefendDisputeResponse,
>;

pub type VerifyRouterData = RouterData<api::Verify, VerifyRequestData, PaymentsResponseData>;

pub type AcceptDisputeRouterData =
    RouterData<api::Accept, AcceptDisputeRequestData, AcceptDisputeResponse>;

pub type SubmitEvidenceRouterData =
    RouterData<api::Evidence, SubmitEvidenceRequestData, SubmitEvidenceResponse>;

pub type UploadFileRouterData = RouterData<api::Upload, UploadFileRequestData, UploadFileResponse>;

pub type RetrieveFileRouterData =
    RouterData<api::Retrieve, RetrieveFileRequestData, RetrieveFileResponse>;

pub type DefendDisputeRouterData =
    RouterData<api::Defend, DefendDisputeRequestData, DefendDisputeResponse>;

#[derive(Debug, Clone)]
pub struct RouterData<Flow, Request, Response> {
    pub flow: PhantomData<Flow>,
    pub merchant_id: String,
    pub customer_id: Option<String>,
    pub connector_customer: Option<String>,
    pub connector: String,
    pub payment_id: String,
    pub attempt_id: String,
    pub status: storage_enums::AttemptStatus,
    pub payment_method: storage_enums::PaymentMethod,
    pub connector_auth_type: ConnectorAuthType,
    pub description: Option<String>,
    pub return_url: Option<String>,
    pub address: PaymentAddress,
    pub auth_type: storage_enums::AuthenticationType,
    pub connector_meta_data: Option<pii::SecretSerdeValue>,
    pub amount_captured: Option<i64>,
    pub access_token: Option<AccessToken>,
    pub session_token: Option<String>,
    pub reference_id: Option<String>,
    pub payment_method_token: Option<String>,
    pub preprocessing_id: Option<String>,

    /// Contains flow-specific data required to construct a request and send it to the connector.
    pub request: Request,

    /// Contains flow-specific data that the connector responds with.
    pub response: Result<Response, ErrorResponse>,

    /// Contains any error response that the connector returns.
    pub payment_method_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PaymentsAuthorizeData {
    pub payment_method_data: payments::PaymentMethodData,
    pub amount: i64,
    pub email: Option<Email>,
    pub currency: storage_enums::Currency,
    pub confirm: bool,
    pub statement_descriptor_suffix: Option<String>,
    pub statement_descriptor: Option<String>,
    pub capture_method: Option<storage_enums::CaptureMethod>,
    pub router_return_url: Option<String>,
    pub webhook_url: Option<String>,
    pub complete_authorize_url: Option<String>,
    // Mandates
    pub setup_future_usage: Option<storage_enums::FutureUsage>,
    pub mandate_id: Option<api_models::payments::MandateIds>,
    pub off_session: Option<bool>,
    pub setup_mandate_details: Option<payments::MandateData>,
    pub browser_info: Option<BrowserInformation>,
    pub order_details: Option<Vec<api_models::payments::OrderDetailsWithAmount>>,
    pub order_category: Option<String>,
    pub session_token: Option<String>,
    pub enrolled_for_3ds: bool,
    pub related_transaction_id: Option<String>,
    pub payment_experience: Option<storage_enums::PaymentExperience>,
    pub payment_method_type: Option<storage_enums::PaymentMethodType>,
    pub customer_id: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct PaymentsCaptureData {
    pub amount_to_capture: i64,
    pub currency: storage_enums::Currency,
    pub connector_transaction_id: String,
    pub payment_amount: i64,
    pub connector_meta: Option<serde_json::Value>,
}

#[derive(Debug, Clone)]
pub struct AuthorizeSessionTokenData {
    pub amount_to_capture: Option<i64>,
    pub currency: storage_enums::Currency,
    pub connector_transaction_id: String,
    pub amount: i64,
}

#[derive(Debug, Clone)]
pub struct ConnectorCustomerData {
    pub description: Option<String>,
    pub email: Option<Email>,
    pub phone: Option<Secret<String>>,
    pub name: Option<String>,
    pub preprocessing_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PaymentMethodTokenizationData {
    pub payment_method_data: payments::PaymentMethodData,
}

#[derive(Debug, Clone)]
pub struct PaymentsPreProcessingData {
    pub payment_method_data: Option<payments::PaymentMethodData>,
    pub amount: Option<i64>,
    pub email: Option<Email>,
    pub currency: Option<storage_enums::Currency>,
    pub payment_method_type: Option<storage_enums::PaymentMethodType>,
}

#[derive(Debug, Clone)]
pub struct CompleteAuthorizeData {
    pub payment_method_data: Option<payments::PaymentMethodData>,
    pub amount: i64,
    pub email: Option<Email>,
    pub currency: storage_enums::Currency,
    pub confirm: bool,
    pub statement_descriptor_suffix: Option<String>,
    pub capture_method: Option<storage_enums::CaptureMethod>,
    // Mandates
    pub setup_future_usage: Option<storage_enums::FutureUsage>,
    pub mandate_id: Option<api_models::payments::MandateIds>,
    pub off_session: Option<bool>,
    pub setup_mandate_details: Option<payments::MandateData>,
    pub redirect_response: Option<CompleteAuthorizeRedirectResponse>,
    pub browser_info: Option<BrowserInformation>,
    pub connector_transaction_id: Option<String>,
    pub connector_meta: Option<serde_json::Value>,
}

#[derive(Debug, Clone)]
pub struct CompleteAuthorizeRedirectResponse {
    pub params: Option<Secret<String>>,
    pub payload: Option<pii::SecretSerdeValue>,
}

#[derive(Debug, Default, Clone)]
pub struct PaymentsSyncData {
    //TODO : add fields based on the connector requirements
    pub connector_transaction_id: ResponseId,
    pub encoded_data: Option<String>,
    pub capture_method: Option<storage_enums::CaptureMethod>,
    pub connector_meta: Option<serde_json::Value>,
    pub mandate_id: Option<api_models::payments::MandateIds>,
}

#[derive(Debug, Default, Clone)]
pub struct PaymentsCancelData {
    pub amount: Option<i64>,
    pub currency: Option<storage_enums::Currency>,
    pub connector_transaction_id: String,
    pub cancellation_reason: Option<String>,
    pub connector_meta: Option<serde_json::Value>,
}

#[derive(Debug, Clone)]
pub struct PaymentsSessionData {
    pub amount: i64,
    pub currency: storage_enums::Currency,
    pub country: Option<api::enums::CountryAlpha2>,
    pub order_details: Option<Vec<api_models::payments::OrderDetailsWithAmount>>,
}

#[derive(Debug, Clone)]
pub struct VerifyRequestData {
    pub currency: storage_enums::Currency,
    pub payment_method_data: payments::PaymentMethodData,
    pub confirm: bool,
    pub statement_descriptor_suffix: Option<String>,
    pub mandate_id: Option<api_models::payments::MandateIds>,
    pub setup_future_usage: Option<storage_enums::FutureUsage>,
    pub off_session: Option<bool>,
    pub setup_mandate_details: Option<payments::MandateData>,
    pub router_return_url: Option<String>,
    pub email: Option<Email>,
    pub return_url: Option<String>,
    pub payment_method_type: Option<storage_enums::PaymentMethodType>,
}

#[derive(Debug, Clone)]
pub struct AccessTokenRequestData {
    pub app_id: String,
    pub id: Option<String>,
    // Add more keys if required
}

pub trait Capturable {
    fn get_capture_amount(&self) -> Option<i64> {
        Some(0)
    }
}

impl Capturable for PaymentsAuthorizeData {
    fn get_capture_amount(&self) -> Option<i64> {
        Some(self.amount)
    }
}

impl Capturable for PaymentsCaptureData {
    fn get_capture_amount(&self) -> Option<i64> {
        Some(self.amount_to_capture)
    }
}

impl Capturable for CompleteAuthorizeData {
    fn get_capture_amount(&self) -> Option<i64> {
        Some(self.amount)
    }
}
impl Capturable for VerifyRequestData {}
impl Capturable for PaymentsCancelData {}
impl Capturable for PaymentsSessionData {}
impl Capturable for PaymentsSyncData {}

pub struct AddAccessTokenResult {
    pub access_token_result: Result<Option<AccessToken>, ErrorResponse>,
    pub connector_supports_access_token: bool,
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
pub struct AccessToken {
    pub token: String,
    pub expires: i64,
}

#[derive(serde::Serialize, Debug, Clone)]
pub struct MandateReference {
    pub connector_mandate_id: Option<String>,
    pub payment_method_id: Option<String>,
}

#[derive(Debug, Clone)]
pub enum PaymentsResponseData {
    TransactionResponse {
        resource_id: ResponseId,
        redirection_data: Option<services::RedirectForm>,
        mandate_reference: Option<MandateReference>,
        connector_metadata: Option<serde_json::Value>,
        network_txn_id: Option<String>,
    },
    SessionResponse {
        session_token: api::SessionToken,
    },
    SessionTokenResponse {
        session_token: String,
    },
    TransactionUnresolvedResponse {
        resource_id: ResponseId,
        //to add more info on cypto response, like `unresolved` reason(overpaid, underpaid, delayed)
        reason: Option<api::enums::UnresolvedResponseReason>,
    },
    TokenizationResponse {
        token: String,
    },

    ConnectorCustomerResponse {
        connector_customer_id: String,
    },

    ThreeDSEnrollmentResponse {
        enrolled_v2: bool,
        related_transaction_id: Option<String>,
    },
    PreProcessingResponse {
        pre_processing_id: PreprocessingResponseId,
        connector_metadata: Option<serde_json::Value>,
        session_token: Option<api::SessionToken>,
    },
}

#[derive(Debug, Clone)]
pub enum PreprocessingResponseId {
    PreProcessingId(String),
    ConnectorTransactionId(String),
}

#[derive(Debug, Clone, Default)]
pub enum ResponseId {
    ConnectorTransactionId(String),
    EncodedData(String),
    #[default]
    NoResponseId,
}

impl ResponseId {
    pub fn get_connector_transaction_id(
        &self,
    ) -> errors::CustomResult<String, errors::ValidationError> {
        match self {
            Self::ConnectorTransactionId(txn_id) => Ok(txn_id.to_string()),
            _ => Err(errors::ValidationError::IncorrectValueProvided {
                field_name: "connector_transaction_id",
            })
            .into_report()
            .attach_printable("Expected connector transaction ID not found"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RefundsData {
    pub refund_id: String,
    pub connector_transaction_id: String,

    pub connector_refund_id: Option<String>,
    pub currency: storage_enums::Currency,
    /// Amount for the payment against which this refund is issued
    pub payment_amount: i64,
    pub reason: Option<String>,
    pub webhook_url: Option<String>,
    /// Amount to be refunded
    pub refund_amount: i64,
    /// Arbitrary metadata required for refund
    pub connector_metadata: Option<serde_json::Value>,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct BrowserInformation {
    pub color_depth: Option<u8>,
    pub java_enabled: Option<bool>,
    pub java_script_enabled: Option<bool>,
    pub language: Option<String>,
    pub screen_height: Option<u32>,
    pub screen_width: Option<u32>,
    pub time_zone: Option<i32>,
    pub ip_address: Option<std::net::IpAddr>,
    pub accept_header: Option<String>,
    pub user_agent: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RefundsResponseData {
    pub connector_refund_id: String,
    pub refund_status: storage_enums::RefundStatus,
    // pub amount_received: Option<i32>, // Calculation for amount received not in place yet
}

#[derive(Debug, Clone, Copy)]
pub enum Redirection {
    Redirect,
    NoRedirect,
}

#[derive(Default, Debug, Clone)]
pub struct AcceptDisputeRequestData {
    pub dispute_id: String,
    pub connector_dispute_id: String,
}

#[derive(Default, Clone, Debug)]
pub struct AcceptDisputeResponse {
    pub dispute_status: api_models::enums::DisputeStatus,
    pub connector_status: Option<String>,
}

#[derive(Default, Debug, Clone)]
pub struct SubmitEvidenceRequestData {
    pub dispute_id: String,
    pub connector_dispute_id: String,
    pub access_activity_log: Option<String>,
    pub billing_address: Option<String>,
    pub cancellation_policy: Option<Vec<u8>>,
    pub cancellation_policy_provider_file_id: Option<String>,
    pub cancellation_policy_disclosure: Option<String>,
    pub cancellation_rebuttal: Option<String>,
    pub customer_communication: Option<Vec<u8>>,
    pub customer_communication_provider_file_id: Option<String>,
    pub customer_email_address: Option<String>,
    pub customer_name: Option<String>,
    pub customer_purchase_ip: Option<String>,
    pub customer_signature: Option<Vec<u8>>,
    pub customer_signature_provider_file_id: Option<String>,
    pub product_description: Option<String>,
    pub receipt: Option<Vec<u8>>,
    pub receipt_provider_file_id: Option<String>,
    pub refund_policy: Option<Vec<u8>>,
    pub refund_policy_provider_file_id: Option<String>,
    pub refund_policy_disclosure: Option<String>,
    pub refund_refusal_explanation: Option<String>,
    pub service_date: Option<String>,
    pub service_documentation: Option<Vec<u8>>,
    pub service_documentation_provider_file_id: Option<String>,
    pub shipping_address: Option<String>,
    pub shipping_carrier: Option<String>,
    pub shipping_date: Option<String>,
    pub shipping_documentation: Option<Vec<u8>>,
    pub shipping_documentation_provider_file_id: Option<String>,
    pub shipping_tracking_number: Option<String>,
    pub invoice_showing_distinct_transactions: Option<Vec<u8>>,
    pub invoice_showing_distinct_transactions_provider_file_id: Option<String>,
    pub recurring_transaction_agreement: Option<Vec<u8>>,
    pub recurring_transaction_agreement_provider_file_id: Option<String>,
    pub uncategorized_file: Option<Vec<u8>>,
    pub uncategorized_file_provider_file_id: Option<String>,
    pub uncategorized_text: Option<String>,
}

#[derive(Default, Clone, Debug)]
pub struct SubmitEvidenceResponse {
    pub dispute_status: api_models::enums::DisputeStatus,
    pub connector_status: Option<String>,
}

#[derive(Default, Debug, Clone)]
pub struct DefendDisputeRequestData {
    pub dispute_id: String,
    pub connector_dispute_id: String,
}

#[derive(Default, Debug, Clone)]
pub struct DefendDisputeResponse {
    pub dispute_status: api_models::enums::DisputeStatus,
    pub connector_status: Option<String>,
}

#[derive(Clone, Debug)]
pub struct UploadFileRequestData {
    pub file_key: String,
    pub file: Vec<u8>,
    pub file_type: mime::Mime,
    pub file_size: i32,
}

#[derive(Default, Clone, Debug)]
pub struct UploadFileResponse {
    pub provider_file_id: String,
}

#[derive(Clone, Debug)]
pub struct RetrieveFileRequestData {
    pub provider_file_id: String,
}

#[derive(Clone, Debug)]
pub struct RetrieveFileResponse {
    pub file_data: Vec<u8>,
}

#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize)]
pub struct ConnectorResponse {
    pub merchant_id: String,
    pub connector: String,
    pub payment_id: String,
    pub amount: i64,
    pub connector_transaction_id: String,
    pub return_url: Option<String>,
    pub three_ds_form: Option<services::RedirectForm>,
}

pub struct ResponseRouterData<Flow, R, Request, Response> {
    pub response: R,
    pub data: RouterData<Flow, Request, Response>,
    pub http_code: u16,
}

// Different patterns of authentication.
#[derive(Default, Debug, Clone, serde::Deserialize)]
#[serde(tag = "auth_type")]
pub enum ConnectorAuthType {
    HeaderKey {
        api_key: String,
    },
    BodyKey {
        api_key: String,
        key1: String,
    },
    SignatureKey {
        api_key: String,
        key1: String,
        api_secret: String,
    },
    MultiAuthKey {
        api_key: String,
        key1: String,
        api_secret: String,
        key2: String,
    },
    #[default]
    NoKey,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConnectorsList {
    pub connectors: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct Response {
    pub headers: Option<http::HeaderMap>,
    pub response: bytes::Bytes,
    pub status_code: u16,
}

#[derive(Clone, Debug)]
pub struct ErrorResponse {
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
    pub status_code: u16,
}

impl ErrorResponse {
    pub fn get_not_implemented() -> Self {
        Self {
            code: errors::ApiErrorResponse::NotImplemented {
                message: errors::api_error_response::NotImplementedMessage::Default,
            }
            .error_code(),
            message: errors::ApiErrorResponse::NotImplemented {
                message: errors::api_error_response::NotImplementedMessage::Default,
            }
            .error_message(),
            reason: None,
            status_code: http::StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
        }
    }
}

impl TryFrom<ConnectorAuthType> for AccessTokenRequestData {
    type Error = errors::ApiErrorResponse;
    fn try_from(connector_auth: ConnectorAuthType) -> Result<Self, Self::Error> {
        match connector_auth {
            ConnectorAuthType::HeaderKey { api_key } => Ok(Self {
                app_id: api_key,
                id: None,
            }),
            ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
                app_id: api_key,
                id: Some(key1),
            }),
            ConnectorAuthType::SignatureKey { api_key, key1, .. } => Ok(Self {
                app_id: api_key,
                id: Some(key1),
            }),
            _ => Err(errors::ApiErrorResponse::InvalidDataValue {
                field_name: "connector_account_details",
            }),
        }
    }
}

impl From<errors::ApiErrorResponse> for ErrorResponse {
    fn from(error: errors::ApiErrorResponse) -> Self {
        Self {
            code: error.error_code(),
            message: error.error_message(),
            reason: None,
            status_code: match error {
                errors::ApiErrorResponse::ExternalConnectorError { status_code, .. } => status_code,
                _ => 500,
            },
        }
    }
}

impl Default for ErrorResponse {
    fn default() -> Self {
        Self::from(errors::ApiErrorResponse::InternalServerError)
    }
}

impl From<&&mut PaymentsAuthorizeRouterData> for AuthorizeSessionTokenData {
    fn from(data: &&mut PaymentsAuthorizeRouterData) -> Self {
        Self {
            amount_to_capture: data.amount_captured,
            currency: data.request.currency,
            connector_transaction_id: data.payment_id.clone(),
            amount: data.request.amount,
        }
    }
}

impl From<&&mut PaymentsAuthorizeRouterData> for ConnectorCustomerData {
    fn from(data: &&mut PaymentsAuthorizeRouterData) -> Self {
        Self {
            email: data.request.email.to_owned(),
            preprocessing_id: data.preprocessing_id.to_owned(),
            description: None,
            phone: None,
            name: None,
        }
    }
}

impl From<&VerifyRouterData> for PaymentsAuthorizeData {
    fn from(data: &VerifyRouterData) -> Self {
        Self {
            currency: data.request.currency,
            payment_method_data: data.request.payment_method_data.clone(),
            confirm: data.request.confirm,
            statement_descriptor_suffix: data.request.statement_descriptor_suffix.clone(),
            mandate_id: data.request.mandate_id.clone(),
            setup_future_usage: data.request.setup_future_usage,
            off_session: data.request.off_session,
            setup_mandate_details: data.request.setup_mandate_details.clone(),
            router_return_url: data.request.router_return_url.clone(),
            email: data.request.email.clone(),
            amount: 0,
            statement_descriptor: None,
            capture_method: None,
            webhook_url: None,
            complete_authorize_url: None,
            browser_info: None,
            order_details: None,
            order_category: None,
            session_token: None,
            enrolled_for_3ds: true,
            related_transaction_id: None,
            payment_experience: None,
            payment_method_type: None,
            customer_id: None,
        }
    }
}

impl<F1, F2, T1, T2> From<(&RouterData<F1, T1, PaymentsResponseData>, T2)>
    for RouterData<F2, T2, PaymentsResponseData>
{
    fn from(item: (&RouterData<F1, T1, PaymentsResponseData>, T2)) -> Self {
        let data = item.0;
        let request = item.1;
        Self {
            flow: PhantomData,
            request,
            merchant_id: data.merchant_id.clone(),
            connector: data.connector.clone(),
            attempt_id: data.attempt_id.clone(),
            status: data.status,
            payment_method: data.payment_method,
            connector_auth_type: data.connector_auth_type.clone(),
            description: data.description.clone(),
            return_url: data.return_url.clone(),
            address: data.address.clone(),
            auth_type: data.auth_type,
            connector_meta_data: data.connector_meta_data.clone(),
            amount_captured: data.amount_captured,
            access_token: data.access_token.clone(),
            response: data.response.clone(),
            payment_method_id: data.payment_method_id.clone(),
            payment_id: data.payment_id.clone(),
            session_token: data.session_token.clone(),
            reference_id: data.reference_id.clone(),
            customer_id: data.customer_id.clone(),
            payment_method_token: None,
            preprocessing_id: None,
            connector_customer: data.connector_customer.clone(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct RequestBody(Secret<String>);

impl RequestBody {
    pub fn log_and_get_request_body<T, F>(
        body: T,
        encoder: F,
    ) -> errors::CustomResult<Self, errors::ParsingError>
    where
        F: FnOnce(T) -> errors::CustomResult<String, errors::ParsingError>,
        T: std::fmt::Debug,
    {
        router_env::logger::info!(connector_request_body=?body);
        Ok(Self(Secret::new(encoder(body)?)))
    }
    pub fn get_inner_value(request_body: Self) -> Secret<String> {
        request_body.0
    }
}

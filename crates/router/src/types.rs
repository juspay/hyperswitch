// FIXME: Why were these data types grouped this way?
//
// Folder `types` is strange for Rust ecosystem, nevertheless it might be okay.
// But folder `enum` is even more strange I unlikely okay. Why should not we introduce folders `type`, `structs` and `traits`? :)
// Is it better to split data types according to business logic instead.
// For example, customers/address/dispute/mandate is "models".
// Separation of concerns instead of separation of forms.

pub mod api;
pub mod storage;
pub mod transformers;

use std::marker::PhantomData;

pub use api_models::enums::Connector;
use common_utils::{pii, pii::Email};
use error_stack::{IntoReport, ResultExt};

use self::{api::payments, storage::enums as storage_enums};
pub use crate::core::payments::PaymentAddress;
use crate::{core::errors, services};

pub type PaymentsAuthorizeRouterData =
    RouterData<api::Authorize, PaymentsAuthorizeData, PaymentsResponseData>;
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
pub type PaymentsCaptureResponseRouterData<R> =
    ResponseRouterData<api::Capture, R, PaymentsCaptureData, PaymentsResponseData>;

pub type RefundsResponseRouterData<F, R> =
    ResponseRouterData<F, R, RefundsData, RefundsResponseData>;

pub type PaymentsAuthorizeType =
    dyn services::ConnectorIntegration<api::Authorize, PaymentsAuthorizeData, PaymentsResponseData>;
pub type PaymentsComeplteAuthorizeType = dyn services::ConnectorIntegration<
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

pub type RefundExecuteType =
    dyn services::ConnectorIntegration<api::Execute, RefundsData, RefundsResponseData>;
pub type RefundSyncType =
    dyn services::ConnectorIntegration<api::RSync, RefundsData, RefundsResponseData>;

pub type RefreshTokenType =
    dyn services::ConnectorIntegration<api::AccessTokenAuth, AccessTokenRequestData, AccessToken>;

pub type VerifyRouterData = RouterData<api::Verify, VerifyRequestData, PaymentsResponseData>;

#[derive(Debug, Clone)]
pub struct RouterData<Flow, Request, Response> {
    pub flow: PhantomData<Flow>,
    pub merchant_id: String,
    pub connector: String,
    pub payment_id: String,
    pub attempt_id: String,
    pub status: storage_enums::AttemptStatus,
    pub payment_method: storage_enums::PaymentMethod,
    pub connector_auth_type: ConnectorAuthType,
    pub description: Option<String>,
    pub return_url: Option<String>,
    pub router_return_url: Option<String>,
    pub complete_authorize_url: Option<String>,
    pub address: PaymentAddress,
    pub auth_type: storage_enums::AuthenticationType,
    pub connector_meta_data: Option<pii::SecretSerdeValue>,
    pub amount_captured: Option<i64>,
    pub access_token: Option<AccessToken>,
    pub session_token: Option<String>,
    pub reference_id: Option<String>,

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
    pub customer_id: Option<String>,
    pub amount: i64,
    pub email: Option<masking::Secret<String, Email>>,
    pub currency: storage_enums::Currency,
    pub confirm: bool,
    pub statement_descriptor_suffix: Option<String>,
    pub statement_descriptor: Option<String>,
    pub capture_method: Option<storage_enums::CaptureMethod>,
    // Mandates
    pub setup_future_usage: Option<storage_enums::FutureUsage>,
    pub mandate_id: Option<api_models::payments::MandateIds>,
    pub off_session: Option<bool>,
    pub setup_mandate_details: Option<payments::MandateData>,
    pub browser_info: Option<BrowserInformation>,
    pub order_details: Option<api_models::payments::OrderDetails>,
    pub session_token: Option<String>,
    pub enrolled_for_3ds: bool,
    pub related_transaction_id: Option<String>,
    pub payment_experience: Option<storage_enums::PaymentExperience>,
    pub payment_method_type: Option<storage_enums::PaymentMethodType>,
}

#[derive(Debug, Clone)]
pub struct PaymentsCaptureData {
    pub amount_to_capture: Option<i64>,
    pub currency: storage_enums::Currency,
    pub connector_transaction_id: String,
    pub amount: i64,
}

#[derive(Debug, Clone)]
pub struct AuthorizeSessionTokenData {
    pub amount_to_capture: Option<i64>,
    pub currency: storage_enums::Currency,
    pub connector_transaction_id: String,
    pub amount: i64,
}

#[derive(Debug, Clone)]
pub struct CompleteAuthorizeData {
    pub payment_method_data: Option<payments::PaymentMethodData>,
    pub amount: i64,
    pub email: Option<masking::Secret<String, Email>>,
    pub currency: storage_enums::Currency,
    pub confirm: bool,
    pub statement_descriptor_suffix: Option<String>,
    pub capture_method: Option<storage_enums::CaptureMethod>,
    // Mandates
    pub setup_future_usage: Option<storage_enums::FutureUsage>,
    pub mandate_id: Option<api_models::payments::MandateIds>,
    pub off_session: Option<bool>,
    pub setup_mandate_details: Option<payments::MandateData>,
    pub browser_info: Option<BrowserInformation>,
    pub connector_transaction_id: Option<String>,
    pub connector_meta: Option<serde_json::Value>,
}

#[derive(Debug, Default, Clone)]
pub struct PaymentsSyncData {
    //TODO : add fields based on the connector requirements
    pub connector_transaction_id: ResponseId,
    pub encoded_data: Option<String>,
    pub capture_method: Option<storage_enums::CaptureMethod>,
    pub connector_meta: Option<serde_json::Value>,
}

#[derive(Debug, Default, Clone)]
pub struct PaymentsCancelData {
    pub amount: Option<i64>,
    pub currency: Option<storage_enums::Currency>,
    pub connector_transaction_id: String,
    pub cancellation_reason: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PaymentsSessionData {
    pub amount: i64,
    pub currency: storage_enums::Currency,
    pub country: Option<String>,
    pub order_details: Option<api_models::payments::OrderDetails>,
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
}

#[derive(Debug, Clone)]
pub struct AccessTokenRequestData {
    pub app_id: String,
    pub id: Option<String>,
    // Add more keys if required
}

pub struct AddAccessTokenResult {
    pub access_token_result: Result<Option<AccessToken>, ErrorResponse>,
    pub connector_supports_access_token: bool,
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
pub struct AccessToken {
    pub token: String,
    pub expires: i64,
}

#[derive(Debug, Clone)]
pub enum PaymentsResponseData {
    TransactionResponse {
        resource_id: ResponseId,
        redirection_data: Option<services::RedirectForm>,
        mandate_reference: Option<String>,
        connector_metadata: Option<serde_json::Value>,
    },
    SessionResponse {
        session_token: api::SessionToken,
    },
    SessionTokenResponse {
        session_token: String,
    },
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
    pub amount: i64,
    pub reason: Option<String>,
    /// Amount to be refunded
    pub refund_amount: i64,
    /// Arbitrary metadata required for refund
    pub connector_metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BrowserInformation {
    pub color_depth: u8,
    pub java_enabled: bool,
    pub java_script_enabled: bool,
    pub language: String,
    pub screen_height: u32,
    pub screen_width: u32,
    pub time_zone: i32,
    pub ip_address: Option<std::net::IpAddr>,
    pub accept_header: String,
    pub user_agent: String,
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
    #[default]
    NoKey,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConnectorsList {
    pub connectors: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct Response {
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

impl<F1, F2, T1, T2> From<(&&mut RouterData<F1, T1, PaymentsResponseData>, T2)>
    for RouterData<F2, T2, PaymentsResponseData>
{
    fn from(item: (&&mut RouterData<F1, T1, PaymentsResponseData>, T2)) -> Self {
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
            router_return_url: data.router_return_url.clone(),
            complete_authorize_url: data.complete_authorize_url.clone(),
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
        }
    }
}

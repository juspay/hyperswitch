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
use common_utils::pii::Email;
use error_stack::{IntoReport, ResultExt};

use self::{api::payments, storage::enums as storage_enums};
pub use crate::core::payments::PaymentAddress;
use crate::{core::errors, services};

pub type PaymentsAuthorizeRouterData =
    RouterData<api::Authorize, PaymentsAuthorizeData, PaymentsResponseData>;
pub type PaymentsSyncRouterData = RouterData<api::PSync, PaymentsSyncData, PaymentsResponseData>;
pub type PaymentsCaptureRouterData =
    RouterData<api::Capture, PaymentsCaptureData, PaymentsResponseData>;
pub type PaymentsCancelRouterData = RouterData<api::Void, PaymentsCancelData, PaymentsResponseData>;
pub type PaymentsSessionRouterData =
    RouterData<api::Session, PaymentsSessionData, PaymentsResponseData>;
pub type RefundsRouterData<F> = RouterData<F, RefundsData, RefundsResponseData>;
pub type RefundExecuteRouterData = RouterData<api::Execute, RefundsData, RefundsResponseData>;
pub type RefundSyncRouterData = RouterData<api::RSync, RefundsData, RefundsResponseData>;

pub type PaymentsResponseRouterData<R> =
    ResponseRouterData<api::Authorize, R, PaymentsAuthorizeData, PaymentsResponseData>;
pub type PaymentsCancelResponseRouterData<R> =
    ResponseRouterData<api::Void, R, PaymentsCancelData, PaymentsResponseData>;
pub type PaymentsSyncResponseRouterData<R> =
    ResponseRouterData<api::PSync, R, PaymentsSyncData, PaymentsResponseData>;
pub type PaymentsSessionResponseRouterData<R> =
    ResponseRouterData<api::Session, R, PaymentsSessionData, PaymentsResponseData>;

pub type RefundsResponseRouterData<F, R> =
    ResponseRouterData<F, R, RefundsData, RefundsResponseData>;

pub type PaymentsAuthorizeType =
    dyn services::ConnectorIntegration<api::Authorize, PaymentsAuthorizeData, PaymentsResponseData>;
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

pub type VerifyRouterData = RouterData<api::Verify, VerifyRequestData, PaymentsResponseData>;

#[derive(Debug, Clone)]
pub struct RouterData<Flow, Request, Response> {
    pub flow: PhantomData<Flow>,
    pub merchant_id: String,
    pub connector: String,
    pub payment_id: String,
    pub status: storage_enums::AttemptStatus,
    pub payment_method: storage_enums::PaymentMethodType,
    pub connector_auth_type: ConnectorAuthType,
    pub description: Option<String>,
    pub return_url: Option<String>,
    pub orca_return_url: Option<String>,
    pub address: PaymentAddress,
    pub auth_type: storage_enums::AuthenticationType,
    pub connector_meta_data: Option<serde_json::Value>,
    pub amount_captured: Option<i64>,

    /// Contains flow-specific data required to construct a request and send it to the connector.
    pub request: Request,

    /// Contains flow-specific data that the connector responds with.
    pub response: Result<Response, ErrorResponse>,

    /// Contains any error response that the connector returns.
    pub payment_method_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PaymentsAuthorizeData {
    pub payment_method_data: payments::PaymentMethod,
    pub amount: i64,
    pub email: Option<masking::Secret<String, Email>>,
    pub currency: storage_enums::Currency,
    pub confirm: bool,
    pub statement_descriptor_suffix: Option<String>,
    // redirect form not used https://juspay.atlassian.net/browse/ORCA-301
    // pub redirection: Option<Redirection>,
    pub capture_method: Option<storage_enums::CaptureMethod>,
    // Mandates
    pub setup_future_usage: Option<storage_enums::FutureUsage>,
    pub mandate_id: Option<api_models::payments::MandateIds>,
    pub off_session: Option<bool>,
    pub setup_mandate_details: Option<payments::MandateData>,
    pub browser_info: Option<BrowserInformation>,
    pub order_details: Option<api_models::payments::OrderDetails>,
}

#[derive(Debug, Clone)]
pub struct PaymentsCaptureData {
    pub amount_to_capture: Option<i64>,
    pub connector_transaction_id: String,
}

#[derive(Debug, Clone)]
pub struct PaymentsSyncData {
    //TODO : add fields based on the connector requirements
    pub connector_transaction_id: ResponseId,
    pub encoded_data: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PaymentsCancelData {
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
    pub payment_method_data: payments::PaymentMethod,
    pub confirm: bool,
    pub statement_descriptor_suffix: Option<String>,
    pub mandate_id: Option<api_models::payments::MandateIds>,
    pub setup_future_usage: Option<storage_enums::FutureUsage>,
    pub off_session: Option<bool>,
    pub setup_mandate_details: Option<payments::MandateData>,
}

#[derive(Debug, Clone)]
pub struct PaymentsTransactionResponse {
    pub resource_id: ResponseId,
    pub redirection_data: Option<services::RedirectForm>,
    pub redirect: bool,
}

#[derive(Debug, Clone)]
pub enum PaymentsResponseData {
    TransactionResponse {
        resource_id: ResponseId,
        redirection_data: Option<services::RedirectForm>,
        redirect: bool,
        mandate_reference: Option<String>,
    },
    SessionResponse {
        session_token: api::SessionToken,
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
    pub payment_method_data: payments::PaymentMethod,
    pub connector_transaction_id: String,
    pub currency: storage_enums::Currency,
    /// Amount for the payment against which this refund is issued
    pub amount: i64,
    /// Amount to be refunded
    pub refund_amount: i64,
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
}

impl ErrorResponse {
    pub fn get_not_implemented() -> Self {
        Self {
            code: errors::ApiErrorResponse::NotImplemented.error_code(),
            message: errors::ApiErrorResponse::NotImplemented.error_message(),
            reason: None,
        }
    }
}

impl From<errors::ApiErrorResponse> for ErrorResponse {
    fn from(error: errors::ApiErrorResponse) -> Self {
        Self {
            code: error.error_code(),
            message: error.error_message(),
            reason: None,
        }
    }
}

impl Default for ErrorResponse {
    fn default() -> Self {
        Self::from(errors::ApiErrorResponse::InternalServerError)
    }
}

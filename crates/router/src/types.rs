// FIXME: Why were these data types grouped this way?
//
// Folder `types` is strange for Rust ecosystem, nevertheless it might be okay.
// But folder `enum` is even more strange I unlikely okay. Why should not we introduce folders `type`, `structs` and `traits`? :)
// Is it better to split data types according to business logic instead.
// For example, customers/address/dispute/mandate is "models".
// Separation of concerns instead of separation of forms.

pub mod api;
pub mod connector;
pub mod storage;

use std::marker::PhantomData;

use error_stack::{IntoReport, ResultExt};

pub use self::connector::Connector;
use self::{api::payments, storage::enums};
pub use crate::core::payments::PaymentAddress;
use crate::{core::errors, services};

pub type PaymentsAuthorizeRouterData =
    RouterData<api::Authorize, PaymentsAuthorizeData, PaymentsResponseData>;
pub type PaymentsSyncRouterData = RouterData<api::PSync, PaymentsSyncData, PaymentsResponseData>;
pub type PaymentsCaptureRouterData =
    RouterData<api::Capture, PaymentsCaptureData, PaymentsResponseData>;
pub type PaymentsCancelRouterData = RouterData<api::Void, PaymentsCancelData, PaymentsResponseData>;
pub type RefundsRouterData<F> = RouterData<F, RefundsData, RefundsResponseData>;

pub type PaymentsResponseRouterData<R> =
    ResponseRouterData<api::Authorize, R, PaymentsAuthorizeData, PaymentsResponseData>;
pub type PaymentsCancelResponseRouterData<R> =
    ResponseRouterData<api::Void, R, PaymentsCancelData, PaymentsResponseData>;
pub type PaymentsSyncResponseRouterData<R> =
    ResponseRouterData<api::PSync, R, PaymentsSyncData, PaymentsResponseData>;
pub type RefundsResponseRouterData<F, R> =
    ResponseRouterData<F, R, RefundsData, RefundsResponseData>;

pub type PaymentsAuthorizeType =
    dyn services::ConnectorIntegration<api::Authorize, PaymentsAuthorizeData, PaymentsResponseData>;
pub type PaymentsSyncType =
    dyn services::ConnectorIntegration<api::PSync, PaymentsSyncData, PaymentsResponseData>;
pub type PaymentsCaptureType =
    dyn services::ConnectorIntegration<api::Capture, PaymentsCaptureData, PaymentsResponseData>;
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
    pub status: enums::AttemptStatus,
    pub payment_method: enums::PaymentMethodType,
    pub connector_auth_type: ConnectorAuthType,
    pub description: Option<String>,
    pub return_url: Option<String>,
    pub orca_return_url: Option<String>,
    pub address: PaymentAddress,
    pub auth_type: enums::AuthenticationType,

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
    pub amount: i32,
    pub currency: enums::Currency,
    pub confirm: bool,
    pub statement_descriptor_suffix: Option<String>,
    // redirect form not used https://juspay.atlassian.net/browse/ORCA-301
    // pub redirection: Option<Redirection>,
    pub capture_method: Option<enums::CaptureMethod>,
    // Mandates
    pub setup_future_usage: Option<enums::FutureUsage>,
    pub mandate_id: Option<String>,
    pub off_session: Option<bool>,
    pub setup_mandate_details: Option<payments::MandateData>,
    pub browser_info: Option<BrowserInformation>,
}

#[derive(Debug, Clone)]
pub struct PaymentsCaptureData {
    pub amount_to_capture: Option<i32>,
    pub connector_transaction_id: String,
}

#[derive(Debug, Clone)]
pub struct PaymentsSyncData {
    //TODO : add fields based on the connector requirements
    pub connector_transaction_id: String,
    pub encoded_data: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PaymentsCancelData {
    pub connector_transaction_id: String,
    pub cancellation_reason: Option<String>,
}

#[derive(Debug, Clone)]
pub struct VerifyRequestData {
    pub payment_method_data: payments::PaymentMethod,
    pub confirm: bool,
    pub statement_descriptor_suffix: Option<String>,
    pub mandate_id: Option<String>,
    pub setup_future_usage: Option<enums::FutureUsage>,
    pub off_session: Option<bool>,
    pub setup_mandate_details: Option<payments::MandateData>,
}
#[derive(Debug, Clone)]
pub struct PaymentsResponseData {
    pub resource_id: ResponseId,
    // pub amount_received: Option<i32>, // Calculation for amount received not in place yet
    pub redirection_data: Option<services::RedirectForm>,
    pub redirect: bool,
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
    pub currency: enums::Currency,
    /// Amount for the payment against which this refund is issued
    pub amount: i32,
    /// Amount to be refunded
    pub refund_amount: i32,
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
    pub refund_status: enums::RefundStatus,
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
    pub amount: i32,
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
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "auth_type")]
pub enum ConnectorAuthType {
    HeaderKey { api_key: String },
    BodyKey { api_key: String, key1: String },
}

impl Default for ConnectorAuthType {
    fn default() -> Self {
        Self::HeaderKey {
            api_key: "".to_string(),
        }
    }
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

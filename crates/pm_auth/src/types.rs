pub mod api;

use std::marker::PhantomData;

use api::auth_service::{BankAccountCredentials, ExchangeToken, LinkToken};
use common_enums::PaymentMethodType;
use masking::Secret;
#[derive(Debug, Clone)]
pub struct PaymentAuthRouterData<F, Request, Response> {
    pub flow: PhantomData<F>,
    pub merchant_id: Option<String>,
    pub connector: Option<String>,
    pub request: Request,
    pub response: Result<Response, ErrorResponse>,
    pub connector_auth_type: ConnectorAuthType,
    pub connector_http_status_code: Option<u16>,
}

#[derive(Debug, Clone)]
pub struct LinkTokenRequest {
    pub client_name: String,
    pub country_codes: Option<Vec<String>>,
    pub language: Option<String>,
    pub user_info: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LinkTokenResponse {
    pub link_token: String,
}

pub type LinkTokenRouterData =
    PaymentAuthRouterData<LinkToken, LinkTokenRequest, LinkTokenResponse>;

#[derive(Debug, Clone)]
pub struct ExchangeTokenRequest {
    pub public_token: String,
}

#[derive(Debug, Clone)]
pub struct ExchangeTokenResponse {
    pub access_token: String,
}

impl From<ExchangeTokenResponse> for api_models::pm_auth::ExchangeTokenCreateResponse {
    fn from(value: ExchangeTokenResponse) -> Self {
        Self {
            access_token: value.access_token,
        }
    }
}

pub type ExchangeTokenRouterData =
    PaymentAuthRouterData<ExchangeToken, ExchangeTokenRequest, ExchangeTokenResponse>;

#[derive(Debug, Clone)]
pub struct BankAccountCredentialsRequest {
    pub access_token: String,
    pub optional_ids: Option<BankAccountOptionalIDs>,
}

#[derive(Debug, Clone)]
pub struct BankAccountOptionalIDs {
    pub ids: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct BankAccountCredentialsResponse {
    pub credentials: Vec<BankAccountDetails>,
}

#[derive(Debug, Clone)]
pub struct BankAccountDetails {
    pub account_name: Option<String>,
    pub account_number: String,
    pub routing_number: String,
    pub payment_method_type: PaymentMethodType,
    pub account_id: String,
    pub account_type: Option<String>,
}

pub type BankDetailsRouterData = PaymentAuthRouterData<
    BankAccountCredentials,
    BankAccountCredentialsRequest,
    BankAccountCredentialsResponse,
>;

pub type PaymentAuthLinkTokenType =
    dyn self::api::ConnectorIntegration<LinkToken, LinkTokenRequest, LinkTokenResponse>;

pub type PaymentAuthExchangeTokenType =
    dyn self::api::ConnectorIntegration<ExchangeToken, ExchangeTokenRequest, ExchangeTokenResponse>;

pub type PaymentAuthBankAccountDetailsType = dyn self::api::ConnectorIntegration<
    BankAccountCredentials,
    BankAccountCredentialsRequest,
    BankAccountCredentialsResponse,
>;

#[derive(Clone, Debug, strum::EnumString, strum::Display)]
#[strum(serialize_all = "snake_case")]
pub enum PaymentMethodAuthConnectors {
    Plaid,
}

#[derive(Debug, Clone)]
pub struct ResponseRouterData<Flow, R, Request, Response> {
    pub response: R,
    pub data: PaymentAuthRouterData<Flow, Request, Response>,
    pub http_code: u16,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct ErrorResponse {
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
    pub status_code: u16,
}

impl ErrorResponse {
    fn get_not_implemented() -> Self {
        Self {
            code: "IR_00".to_string(),
            message: "This API is under development and will be made available soon.".to_string(),
            reason: None,
            status_code: http::StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
        }
    }
}

#[derive(Default, Debug, Clone, serde::Deserialize)]
pub enum ConnectorAuthType {
    BodyKey {
        client_id: Secret<String>,
        secret: Secret<String>,
    },
    #[default]
    NoKey,
}

#[derive(Clone, Debug)]
pub struct Response {
    pub headers: Option<http::HeaderMap>,
    pub response: bytes::Bytes,
    pub status_code: u16,
}

#[derive(serde::Deserialize, Clone)]
pub struct AuthServiceQueryParam {
    pub client_secret: Option<String>,
}

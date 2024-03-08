use api_models::enums;

use super::BoxedConnector;

#[derive(Debug, Clone)]
pub struct PreAuthentication;

#[derive(Debug, Clone)]
pub struct Authentication;

#[derive(Debug, Clone)]
pub struct PostAuthentication;
use crate::{services, types};

#[derive(Clone, serde::Deserialize, Debug, serde::Serialize)]
pub struct AcquirerDetails {
    pub acquirer_bin: String,
    pub acquirer_merchant_mid: String,
}

#[derive(Clone, serde::Deserialize, Debug, serde::Serialize)]
pub struct AuthenticationResponse {
    pub trans_status: api_models::payments::TransactionStatus,
    pub acs_url: Option<url::Url>,
    pub challenge_request: Option<String>,
    pub acs_reference_number: Option<String>,
    pub acs_trans_id: Option<String>,
    pub three_dsserver_trans_id: Option<String>,
    pub acs_signed_content: Option<String>,
}

#[derive(Clone, serde::Deserialize, Debug, serde::Serialize)]
pub struct PostAuthenticationResponse {
    pub trans_status: String,
    pub authentication_value: Option<String>,
    pub eci: Option<String>,
}

#[derive(Clone, serde::Deserialize, Debug, serde::Serialize, PartialEq, Eq)]
pub enum MessageCategory {
    Payment,
    NonPayment,
}

pub trait ConnectorAuthentication:
    services::ConnectorIntegration<
    Authentication,
    types::authentication::ConnectorAuthenticationRequestData,
    types::authentication::AuthenticationResponseData,
>
{
}

pub trait ConnectorPreAuthentication:
    services::ConnectorIntegration<
    PreAuthentication,
    types::authentication::PreAuthNRequestData,
    types::authentication::AuthenticationResponseData,
>
{
}

pub trait ConnectorPostAuthentication:
    services::ConnectorIntegration<
    PostAuthentication,
    types::authentication::ConnectorPostAuthenticationRequestData,
    types::authentication::AuthenticationResponseData,
>
{
}

pub trait ExternalAuthentication:
    super::ConnectorCommon
    + ConnectorAuthentication
    + ConnectorPreAuthentication
    + ConnectorPostAuthentication
{
}

#[derive(Clone, Debug)]
pub struct AuthenticationConnectorData {
    pub connector: BoxedConnector,
    pub connector_name: enums::AuthenticationConnectors,
}

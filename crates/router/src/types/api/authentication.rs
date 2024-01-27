use crate::{services, types};

#[derive(Clone, serde::Deserialize, Debug, serde::Serialize)]
pub struct AcquirerDetails {
    pub acquirer_bin: String,
    pub acquirer_merchant_mid: String,
}

#[derive(Clone, serde::Deserialize, Debug, serde::Serialize)]
pub struct AuthenticationResponse {
    pub trans_status: String,
    pub acs_url: Option<url::Url>,
    pub challenge_request: Option<String>,
}

#[derive(Clone, serde::Deserialize, Debug, serde::Serialize, PartialEq, Eq)]
pub enum MessageCategory {
    Payment,
    NonPayment,
}

#[derive(Debug, Clone)]
pub struct Authentication;

pub trait ConnectorAuthentication:
    services::ConnectorIntegration<
    Authentication,
    types::ConnectorAuthenticationRequestData,
    types::ConnectorAuthenticationResponse,
>
{
}

pub trait ExternalAuthentication: super::ConnectorCommon + ConnectorAuthentication {}

use std::str::FromStr;

use api_models::enums;
use common_utils::errors::CustomResult;
use error_stack::{IntoReport, ResultExt};

use super::BoxedConnector;
use crate::core::errors;

#[derive(Debug, Clone)]
pub struct PreAuthentication;

#[derive(Debug, Clone)]
pub struct Authentication;

#[derive(Debug, Clone)]
pub struct PostAuthentication;
use crate::{connector, services, types, types::storage};

#[derive(Clone, serde::Deserialize, Debug, serde::Serialize)]
pub struct AcquirerDetails {
    pub acquirer_bin: String,
    pub acquirer_merchant_mid: String,
}

#[derive(Clone, serde::Deserialize, Debug, serde::Serialize)]
pub struct AuthenticationResponse {
    pub trans_status: common_enums::TransactionStatus,
    pub acs_url: Option<url::Url>,
    pub challenge_request: Option<String>,
    pub acs_reference_number: Option<String>,
    pub acs_trans_id: Option<String>,
    pub three_dsserver_trans_id: Option<String>,
    pub acs_signed_content: Option<String>,
}

impl TryFrom<storage::Authentication> for AuthenticationResponse {
    type Error = error_stack::Report<errors::ApiErrorResponse>;
    fn try_from(authentication: storage::Authentication) -> Result<Self, Self::Error> {
        let trans_status = authentication.trans_status.ok_or(errors::ApiErrorResponse::InternalServerError.into()).attach_printable("trans_status must be populated in authentication table authentication call is successful")?;
        let acs_url = authentication
            .acs_url
            .map(|url| url::Url::from_str(&url))
            .transpose()
            .into_report()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("not a valid URL")?;
        Ok(Self {
            trans_status,
            acs_url,
            challenge_request: authentication.challenge_request,
            acs_reference_number: authentication.acs_reference_number,
            acs_trans_id: authentication.acs_trans_id,
            three_dsserver_trans_id: authentication.three_ds_server_trans_id,
            acs_signed_content: authentication.acs_signed_content,
        })
    }
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

impl AuthenticationConnectorData {
    pub fn get_connector_by_name(name: &str) -> CustomResult<Self, errors::ApiErrorResponse> {
        let connector_name = enums::AuthenticationConnectors::from_str(name)
            .into_report()
            .change_context(errors::ApiErrorResponse::IncorrectConnectorNameGiven)
            .attach_printable_lazy(|| format!("unable to parse connector: {name}"))?;
        let connector = Self::convert_connector(connector_name)?;
        Ok(Self {
            connector,
            connector_name,
        })
    }

    fn convert_connector(
        connector_name: enums::AuthenticationConnectors,
    ) -> CustomResult<BoxedConnector, errors::ApiErrorResponse> {
        match connector_name {
            enums::AuthenticationConnectors::Threedsecureio => {
                Ok(Box::new(&connector::Threedsecureio))
            }
        }
    }
}

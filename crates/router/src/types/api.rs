pub mod admin;
pub mod api_keys;
pub mod configs;
pub mod customers;
pub mod enums;
pub mod mandates;
pub mod payment_methods;
pub mod payments;
pub mod refunds;
pub mod webhooks;

use std::{fmt::Debug, str::FromStr};

use error_stack::{report, IntoReport, ResultExt};

pub use self::{
    admin::*, api_keys::*, configs::*, customers::*, payment_methods::*, payments::*, refunds::*,
    webhooks::*,
};
use super::ErrorResponse;
use crate::{
    configs::settings::Connectors,
    connector, consts,
    core::errors::{self, CustomResult},
    services::{ConnectorIntegration, ConnectorRedirectResponse},
    types::{self, api::enums as api_enums},
};

#[derive(Clone, Debug)]
pub struct AccessTokenAuth;

pub trait ConnectorAccessToken:
    ConnectorIntegration<AccessTokenAuth, types::AccessTokenRequestData, types::AccessToken>
{
}

pub trait ConnectorCommon {
    /// Name of the connector (in lowercase).
    fn id(&self) -> &'static str;

    /// HTTP header used for authorization.
    fn get_auth_header(
        &self,
        _auth_type: &types::ConnectorAuthType,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        Ok(Vec::new())
    }

    /// HTTP `Content-Type` to be used for POST requests.
    /// Defaults to `application/json`.
    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    // FIXME write doc - think about this
    // fn headers(&self) -> Vec<(&str, &str)>;

    /// The base URL for interacting with the connector's API.
    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str;

    /// common error response for a connector if it is same in all case
    fn build_error_response(
        &self,
        res: types::Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        Ok(ErrorResponse {
            status_code: res.status_code,
            code: consts::NO_ERROR_CODE.to_string(),
            message: consts::NO_ERROR_MESSAGE.to_string(),
            reason: None,
        })
    }
}

/// Extended trait for connector common to allow functions with generic type
pub trait ConnectorCommonExt<Flow, Req, Resp>:
    ConnectorCommon + ConnectorIntegration<Flow, Req, Resp>
{
    /// common header builder when every request for the connector have same headers
    fn build_headers(
        &self,
        _req: &types::RouterData<Flow, Req, Resp>,
        _connectors: &Connectors,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        Ok(Vec::new())
    }
}

pub trait Router {}

pub trait Connector:
    Send + Refund + Payment + Debug + ConnectorRedirectResponse + IncomingWebhook + ConnectorAccessToken
{
}

pub struct Re;

pub struct Pe;

impl<
        T: Refund
            + Payment
            + Debug
            + ConnectorRedirectResponse
            + Send
            + IncomingWebhook
            + ConnectorAccessToken,
    > Connector for T
{
}

type BoxedConnector = Box<&'static (dyn Connector + Sync)>;

// Normal flow will call the connector and follow the flow specific operations (capture, authorize)
// SessionTokenFromMetadata will avoid calling the connector instead create the session token ( for sdk )
pub enum GetToken {
    Metadata,
    Connector,
}

pub struct ConnectorData {
    pub connector: BoxedConnector,
    pub connector_name: types::Connector,
    pub get_token: GetToken,
}

pub enum ConnectorCallType {
    Routing,
    Multiple(Vec<ConnectorData>),
    Single(ConnectorData),
}

impl ConnectorCallType {
    pub fn is_single(&self) -> bool {
        matches!(self, Self::Single(_))
    }
}

impl ConnectorData {
    pub fn get_connector_by_name(
        connectors: &Connectors,
        name: &str,
        connector_type: GetToken,
    ) -> CustomResult<Self, errors::ApiErrorResponse> {
        let connector = Self::convert_connector(connectors, name)?;
        let connector_name = api_enums::Connector::from_str(name)
            .into_report()
            .change_context(errors::ConnectorError::InvalidConnectorName)
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable_lazy(|| format!("unable to parse connector name {connector:?}"))?;
        Ok(Self {
            connector,
            connector_name,
            get_token: connector_type,
        })
    }

    fn convert_connector(
        _connectors: &Connectors,
        connector_name: &str,
    ) -> CustomResult<BoxedConnector, errors::ApiErrorResponse> {
        match connector_name {
            "aci" => Ok(Box::new(&connector::Aci)),
            "adyen" => Ok(Box::new(&connector::Adyen)),
            "applepay" => Ok(Box::new(&connector::Applepay)),
            "authorizedotnet" => Ok(Box::new(&connector::Authorizedotnet)),
            "braintree" => Ok(Box::new(&connector::Braintree)),
            "checkout" => Ok(Box::new(&connector::Checkout)),
            "cybersource" => Ok(Box::new(&connector::Cybersource)),
            "fiserv" => Ok(Box::new(&connector::Fiserv)),
            "globalpay" => Ok(Box::new(&connector::Globalpay)),
            "klarna" => Ok(Box::new(&connector::Klarna)),
            "payu" => Ok(Box::new(&connector::Payu)),
            "rapyd" => Ok(Box::new(&connector::Rapyd)),
            "shift4" => Ok(Box::new(&connector::Shift4)),
            "stripe" => Ok(Box::new(&connector::Stripe)),
            "worldline" => Ok(Box::new(&connector::Worldline)),
            "worldpay" => Ok(Box::new(&connector::Worldpay)),
            "dlocal" => Ok(Box::new(&connector::Dlocal)),
            _ => Err(report!(errors::ConnectorError::InvalidConnectorName)
                .attach_printable(format!("invalid connector name: {connector_name}")))
            .change_context(errors::ApiErrorResponse::InternalServerError),
        }
    }
}

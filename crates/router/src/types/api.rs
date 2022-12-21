pub mod admin;
pub mod customers;
pub mod enums;
pub mod mandates;
pub mod payment_methods;
pub mod payments;
pub mod refunds;
pub mod webhooks;

use std::{fmt::Debug, marker, str::FromStr};

use error_stack::{report, IntoReport, ResultExt};

pub use self::{admin::*, customers::*, payment_methods::*, payments::*, refunds::*, webhooks::*};
use crate::{
    configs::settings::Connectors,
    connector,
    core::errors::{self, CustomResult},
    services::ConnectorRedirectResponse,
    types::{self, api::enums as api_enums},
};

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
}

pub trait Router {}

pub trait Connector:
    Send + Refund + Payment + Debug + ConnectorRedirectResponse + IncomingWebhook
{
}

pub struct Re;

pub struct Pe;

impl<T: Refund + Payment + Debug + ConnectorRedirectResponse + Send + IncomingWebhook> Connector
    for T
{
}

type BoxedConnector = Box<&'static (dyn Connector + marker::Sync)>;

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
    Single(ConnectorData),
    Multiple(Vec<ConnectorData>),
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
            .attach_printable_lazy(|| format!("unable to parse connector name {connector:?}"))
            .change_context(errors::ApiErrorResponse::InternalServerError)?;
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
            "stripe" => Ok(Box::new(&connector::Stripe)),
            "adyen" => Ok(Box::new(&connector::Adyen)),
            "aci" => Ok(Box::new(&connector::Aci)),
            "checkout" => Ok(Box::new(&connector::Checkout)),
            "authorizedotnet" => Ok(Box::new(&connector::Authorizedotnet)),
            "braintree" => Ok(Box::new(&connector::Braintree)),
            "klarna" => Ok(Box::new(&connector::Klarna)),
            "applepay" => Ok(Box::new(&connector::Applepay)),
            _ => Err(report!(errors::UnexpectedError)
                .attach_printable(format!("invalid connector name: {connector_name}")))
            .change_context(errors::ConnectorError::InvalidConnectorName)
            .change_context(errors::ApiErrorResponse::InternalServerError),
        }
    }
}

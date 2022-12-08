pub mod admin;
pub mod customers;
pub mod mandates;
pub mod payment_methods;
pub mod payments;
pub mod refunds;
pub mod webhooks;

use std::{fmt::Debug, marker, str::FromStr};

use error_stack::{report, IntoReport, ResultExt};

pub use self::{
    admin::*, customers::*, payment_methods::*, payments::*, refunds::*, types::enums::FutureUsage,
    webhooks::*,
};
use super::{storage, ConnectorsList};
use crate::{
    configs::settings::Connectors,
    connector,
    core::errors::{self, CustomResult},
    services::ConnectorRedirectResponse,
    types::{self, api},
    utils::{OptionExt, ValueExt},
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

    // TODO: Pass the connectors as borrow
    /// The base URL for interacting with the connector's API.
    fn base_url(&self, connectors: Connectors) -> String;
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

pub struct ConnectorData {
    pub connector: BoxedConnector,
    pub connector_name: types::Connector,
}

impl ConnectorData {
    pub fn construct(
        connectors: &Connectors,
        merchant_account: &storage::MerchantAccount,
    ) -> CustomResult<ConnectorData, errors::ApiErrorResponse> {
        // Add Validate also to ParseStruct
        //FIXME: Need Proper Routing Logic
        let vec_val: Vec<serde_json::Value> = merchant_account
            .custom_routing_rules
            .as_ref()
            .cloned()
            .parse_value("CustomRoutingRulesVec")
            .change_context(errors::ConnectorError::RoutingRulesParsingError)
            .change_context(errors::ApiErrorResponse::InternalServerError)?;
        let custom_routing_rules: api::CustomRoutingRules = vec_val[0]
            .clone()
            .parse_value("CustomRoutingRules")
            .change_context(errors::ConnectorError::RoutingRulesParsingError)
            .change_context(errors::ApiErrorResponse::InternalServerError)?;
        let connector_names = custom_routing_rules
            .connectors_pecking_order
            .unwrap_or_else(|| vec!["stripe".to_string()]);

        //use routing rules if configured by merchant else query MCA as per PM
        let connector_list: ConnectorsList = ConnectorsList {
            connectors: connector_names,
        };

        let connector_name = connector_list
            .connectors
            .first()
            .get_required_value("connectors")
            .change_context(errors::ConnectorError::FailedToObtainPreferredConnector)
            .change_context(errors::ApiErrorResponse::InternalServerError)?
            .as_str();

        Self::get_connector_by_name(connectors, connector_name)
    }

    pub fn get_connector_by_name(
        connectors: &Connectors,
        name: &str,
    ) -> CustomResult<ConnectorData, errors::ApiErrorResponse> {
        let connector = Self::convert_connector(connectors, name)?;
        let connector_name = types::Connector::from_str(name)
            .into_report()
            .change_context(errors::ConnectorError::InvalidConnectorName)
            .attach_printable_lazy(|| format!("unable to parse connector name {:?}", connector))
            .change_context(errors::ApiErrorResponse::InternalServerError)?;
        Ok(ConnectorData {
            connector,
            connector_name,
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
            _ => Err(report!(errors::UnexpectedError)
                .attach_printable(format!("invalid connector name: {connector_name}")))
            .change_context(errors::ConnectorError::InvalidConnectorName)
            .change_context(errors::ApiErrorResponse::InternalServerError),
        }
    }
}

use std::str::FromStr;

use api_models::enums;
use common_utils::errors::CustomResult;
use error_stack::{IntoReport, ResultExt};

use super::{BoxedConnector, ConnectorData, SessionConnectorData};
use crate::{
    connector,
    core::errors,
    services::api,
    types::fraud_check::{
        FraudCheckCheckoutData, FraudCheckFulfillmentData, FraudCheckRecordReturnData,
        FraudCheckResponseData, FraudCheckSaleData, FraudCheckTransactionData,
    },
};

#[derive(Debug, Clone)]
pub struct Sale;

pub trait FraudCheckSale:
    api::ConnectorIntegration<Sale, FraudCheckSaleData, FraudCheckResponseData>
{
}

#[derive(Debug, Clone)]
pub struct Checkout;

pub trait FraudCheckCheckout:
    api::ConnectorIntegration<Checkout, FraudCheckCheckoutData, FraudCheckResponseData>
{
}

#[derive(Debug, Clone)]
pub struct Transaction;

pub trait FraudCheckTransaction:
    api::ConnectorIntegration<Transaction, FraudCheckTransactionData, FraudCheckResponseData>
{
}

#[derive(Debug, Clone)]
pub struct Fulfillment;

pub trait FraudCheckFulfillment:
    api::ConnectorIntegration<Fulfillment, FraudCheckFulfillmentData, FraudCheckResponseData>
{
}

#[derive(Debug, Clone)]
pub struct RecordReturn;

pub trait FraudCheckRecordReturn:
    api::ConnectorIntegration<RecordReturn, FraudCheckRecordReturnData, FraudCheckResponseData>
{
}

#[derive(Clone, Debug)]
pub struct FraudCheckConnectorData {
    pub connector: BoxedConnector,
    pub connector_name: enums::FrmConnectors,
}
pub enum ConnectorCallType {
    PreDetermined(ConnectorData),
    Retryable(Vec<ConnectorData>),
    SessionMultiple(Vec<SessionConnectorData>),
}

impl FraudCheckConnectorData {
        /// Retrieves a connector by its name and returns a CustomResult containing the connector and any potential errors.
    pub fn get_connector_by_name(name: &str) -> CustomResult<Self, errors::ApiErrorResponse> {
        let connector_name = enums::FrmConnectors::from_str(name)
            .into_report()
            .change_context(errors::ApiErrorResponse::IncorrectConnectorNameGiven)
            .attach_printable_lazy(|| {
                format!("unable to parse connector: {:?}", name.to_string())
            })?;
        let connector = Self::convert_connector(connector_name)?;
        Ok(Self {
            connector,
            connector_name,
        })
    }

        /// Converts the given connector name into a BoxedConnector and returns a CustomResult.
    ///
    /// # Arguments
    ///
    /// * `connector_name` - An enums::FrmConnectors value representing the connector name.
    ///
    /// # Returns
    ///
    /// A CustomResult containing a BoxedConnector if the connector name is valid, otherwise an errors::ApiErrorResponse.
    ///
    fn convert_connector(
        connector_name: enums::FrmConnectors,
    ) -> CustomResult<BoxedConnector, errors::ApiErrorResponse> {
        match connector_name {
            enums::FrmConnectors::Signifyd => Ok(Box::new(&connector::Signifyd)),
            enums::FrmConnectors::Riskified => Ok(Box::new(&connector::Riskified)),
        }
    }
}

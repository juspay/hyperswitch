use std::str::FromStr;

use api_models::enums;
use common_utils::errors::CustomResult;
use error_stack::ResultExt;
pub use hyperswitch_domain_models::router_flow_types::fraud_check::{
    Checkout, Fulfillment, RecordReturn, Sale, Transaction,
};

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

pub trait FraudCheckSale:
    api::ConnectorIntegration<Sale, FraudCheckSaleData, FraudCheckResponseData>
{
}

pub trait FraudCheckCheckout:
    api::ConnectorIntegration<Checkout, FraudCheckCheckoutData, FraudCheckResponseData>
{
}

pub trait FraudCheckTransaction:
    api::ConnectorIntegration<Transaction, FraudCheckTransactionData, FraudCheckResponseData>
{
}

pub trait FraudCheckFulfillment:
    api::ConnectorIntegration<Fulfillment, FraudCheckFulfillmentData, FraudCheckResponseData>
{
}

pub trait FraudCheckRecordReturn:
    api::ConnectorIntegration<RecordReturn, FraudCheckRecordReturnData, FraudCheckResponseData>
{
}

#[derive(Clone)]
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
    pub fn get_connector_by_name(name: &str) -> CustomResult<Self, errors::ApiErrorResponse> {
        let connector_name = enums::FrmConnectors::from_str(name)
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

    fn convert_connector(
        connector_name: enums::FrmConnectors,
    ) -> CustomResult<BoxedConnector, errors::ApiErrorResponse> {
        match connector_name {
            enums::FrmConnectors::Signifyd => Ok(Box::new(&connector::Signifyd)),
            enums::FrmConnectors::Riskified => Ok(Box::new(&connector::Riskified)),
        }
    }
}

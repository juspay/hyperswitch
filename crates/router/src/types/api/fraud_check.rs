use std::str::FromStr;

use api_models::enums;
use common_utils::errors::CustomResult;
use error_stack::ResultExt;
pub use hyperswitch_domain_models::router_flow_types::fraud_check::{
    Checkout, Fulfillment, RecordReturn, Sale, Transaction,
};
pub use hyperswitch_interfaces::api::fraud_check::{
    FraudCheckCheckout, FraudCheckFulfillment, FraudCheckRecordReturn, FraudCheckSale,
    FraudCheckTransaction,
};

pub use super::fraud_check_v2::{
    FraudCheckCheckoutV2, FraudCheckFulfillmentV2, FraudCheckRecordReturnV2, FraudCheckSaleV2,
    FraudCheckTransactionV2, FraudCheckV2,
};
use super::{ConnectorData, SessionConnectorData};
use crate::{connector, core::errors, services::connector_integration_interface::ConnectorEnum};

#[derive(Clone)]
pub struct FraudCheckConnectorData {
    pub connector: ConnectorEnum,
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
    ) -> CustomResult<ConnectorEnum, errors::ApiErrorResponse> {
        match connector_name {
            enums::FrmConnectors::Signifyd => {
                Ok(ConnectorEnum::Old(Box::new(&connector::Signifyd)))
            }
            enums::FrmConnectors::Riskified => {
                Ok(ConnectorEnum::Old(Box::new(connector::Riskified::new())))
            }
        }
    }
}

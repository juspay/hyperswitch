use std::str::FromStr;

use error_stack::{IntoReport, ResultExt};
use pm_auth::{
    connector::plaid,
    types::{
        self as pm_auth_types,
        api::{BoxedPaymentAuthConnector, PaymentAuthConnectorData},
    },
};

use crate::core::{
    errors::{self, ApiErrorResponse},
    pm_auth::helpers::PaymentAuthConnectorDataExt,
};

impl PaymentAuthConnectorDataExt for PaymentAuthConnectorData {
    fn get_connector_by_name(name: &str) -> errors::CustomResult<Self, ApiErrorResponse> {
        let connector_name = pm_auth_types::PaymentMethodAuthConnectors::from_str(name)
            .into_report()
            .change_context(ApiErrorResponse::IncorrectConnectorNameGiven)
            .attach_printable_lazy(|| {
                format!("unable to parse connector: {:?}", name.to_string())
            })?;
        let connector = Self::convert_connector(connector_name.clone())?;
        Ok(Self {
            connector,
            connector_name,
        })
    }
    fn convert_connector(
        connector_name: pm_auth_types::PaymentMethodAuthConnectors,
    ) -> errors::CustomResult<BoxedPaymentAuthConnector, ApiErrorResponse> {
        match connector_name {
            pm_auth_types::PaymentMethodAuthConnectors::Plaid => Ok(Box::new(&plaid::Plaid)),
        }
    }
}

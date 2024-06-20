use common_utils::ext_traits::ValueExt;
use error_stack::ResultExt;
use pm_auth::types::{self as pm_auth_types, api::BoxedPaymentAuthConnector};

use crate::{
    core::errors::{self, ApiErrorResponse},
    types::{self, domain, transformers::ForeignTryFrom},
};

pub trait PaymentAuthConnectorDataExt {
    fn get_connector_by_name(name: &str) -> errors::CustomResult<Self, ApiErrorResponse>
    where
        Self: Sized;
    fn convert_connector(
        connector_name: pm_auth_types::PaymentMethodAuthConnectors,
    ) -> errors::CustomResult<BoxedPaymentAuthConnector, ApiErrorResponse>;
}

pub fn get_connector_auth_type(
    merchant_connector_account: domain::MerchantConnectorAccount,
) -> errors::CustomResult<pm_auth_types::ConnectorAuthType, ApiErrorResponse> {
    let auth_type: types::ConnectorAuthType = merchant_connector_account
        .connector_account_details
        .parse_value("ConnectorAuthType")
        .change_context(ApiErrorResponse::MerchantConnectorAccountNotFound {
            id: "ConnectorAuthType".to_string(),
        })?;

    pm_auth_types::ConnectorAuthType::foreign_try_from(auth_type)
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable("Failed while converting ConnectorAuthType")
}

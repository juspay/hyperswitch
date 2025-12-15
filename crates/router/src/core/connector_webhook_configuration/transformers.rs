use common_utils::ext_traits::{OptionExt, ValueExt};
use error_stack::ResultExt;
use router_env::tracing::{self, instrument};
 use crate::routes::app::SessionStateInfo;

use crate::{
    core::{
        errors::RouterResult, 
        payments::helpers, utils as core_utils,
    },
    errors,
    types::{
        ConnectorWebhookRegisterData,
        domain,
        fraud_check::{FraudCheckFulfillmentData, FrmFulfillmentRouterData},
        storage, ConnectorAuthType, ErrorResponse, PaymentAddress, RouterData,
    },
    utils, SessionState,
};
use api_models::admin::ConnectorWebhookRegisterRequest;

#[cfg(feature = "v2")]
pub async fn construct_fulfillment_router_data<'a>(
    _state: &'a SessionState,
    _platform: &domain::Platform,
    _merchant_connector_account: storage::MerchantConnectorAccount,
    _webhook_register_request: ConnectorWebhookRegisterRequest,
) -> RouterResult<ConnectorWebhookRegisterData> {
    todo!()
}

#[cfg(feature = "v1")]
#[instrument(skip_all)]
pub async fn construct_webhook_register_request_data<'a>(
    state: &'a SessionState,
    merchant_connector_account: domain::MerchantConnectorAccount,
    webhook_register_request: ConnectorWebhookRegisterRequest,
) -> RouterResult<ConnectorWebhookRegisterData> {
    let merchant_id = merchant_connector_account.merchant_id.get_string_repr();
    let merchant_connector_id = merchant_connector_account.merchant_connector_id.get_string_repr();
    let router_base_url = state.base_url.clone();
    Ok(ConnectorWebhookRegisterData {
        webhook_url: format!(
            "{router_base_url}/webhooks/{merchant_id}/{merchant_connector_id}",
        ),
        event_type: webhook_register_request.event_type,
    })
}

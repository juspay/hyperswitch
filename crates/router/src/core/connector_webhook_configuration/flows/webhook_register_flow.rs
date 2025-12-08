use common_utils::ext_traits::{OptionExt, ValueExt};
use error_stack::ResultExt;
use router_env::tracing::{self, instrument};

use crate::{
    core::{
        errors::RouterResult, admin::ConnectorWebhookRegisterRequest,
        payments::helpers, utils as core_utils,
    },
    errors,
    types::{
        domain,
        fraud_check::{FraudCheckFulfillmentData, FrmFulfillmentRouterData},
        storage, ConnectorAuthType, ErrorResponse, PaymentAddress, RouterData,
    },
    utils, SessionState,
};

#[cfg(feature = "v2")]
pub async fn construct_fulfillment_router_data<'a>(
    _state: &'a SessionState,
    _platform: &domain::Platform,
    _merchant_connector_account: MerchantConnectorAccount,
    _webhook_register_request: ConnectorWebhookRegisterRequest,
) -> RouterResult<ConnectorWebhookRegisterData> {
    todo!()
}

#[cfg(feature = "v1")]
#[instrument(skip_all)]
pub async fn construct_webhook_register_request_data<'a>(
    state: &'a SessionState,
    merchant_connector_account: MerchantConnectorAccount,
    webhook_register_request: ConnectorWebhookRegisterRequest,
) -> RouterResult<ConnectorWebhookRegisterData> {
    let merchant_id = merchant_connector_account.merchant_id;
    let merchant_connector_id = merchant_connector_account.merchant_connector_id;
    let router_base_url = state.config().base_url.clone();
    Ok(ConnectorWebhookRegisterData {
        webhook_url: format!(
            "{router_base_url}/webhooks/{merchant_id}/{merchant_connector_id}",
        ),
        event_type: webhook_register_request.event_type,
    })
}

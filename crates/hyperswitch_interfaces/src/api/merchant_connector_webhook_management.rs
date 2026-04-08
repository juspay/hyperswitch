//! Connector Webhooks Interface for V1
use hyperswitch_domain_models::{
    router_flow_types::merchant_connector_webhook_management::ConnectorWebhookRegister,
    router_request_types::merchant_connector_webhook_management::ConnectorWebhookRegisterRequest,
    router_response_types::merchant_connector_webhook_management::ConnectorWebhookRegisterResponse,
};

use super::{ConnectorCommon, ConnectorIntegration};

/// trait WebhookRegister for V1
pub trait WebhookRegister:
    ConnectorIntegration<
    ConnectorWebhookRegister,
    ConnectorWebhookRegisterRequest,
    ConnectorWebhookRegisterResponse,
>
{
}

/// trait ConfigureConnectorWebhook for V1
pub trait ConfigureConnectorWebhook: ConnectorCommon + WebhookRegister {}

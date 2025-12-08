//! Connector Webhooks Interface for V1

use hyperswitch_domain_models::{
    router_flow_types::{
        configure_connector_webhook::{
            ConnectorWebhookRegister
        },
    },
    router_request_types::{
        configure_connector_webhook::ConnectorWebhookRegisterData
    },
    router_response_types::{
        configure_connector_webhook::
           ConnectorWebhookRegisterResponse
    },
};

use super::{
    ConnectorCommon, ConnectorIntegration,
};

/// trait WebhookRegister for V1
pub trait WebhookRegister:
    ConnectorIntegration<
    ConnectorWebhookRegister,
    ConnectorWebhookRegisterData,
    ConnectorWebhookRegisterResponse,
>
{
}


/// trait ConfigureConnectorWebhook for V1
pub trait ConfigureConnectorWebhook:
    ConnectorCommon
    + WebhookRegister
{
}

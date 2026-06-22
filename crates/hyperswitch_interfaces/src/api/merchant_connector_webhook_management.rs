//! Connector Webhooks Interface for V1
use hyperswitch_domain_models::{
    router_flow_types::merchant_connector_webhook_management::{
        ConnectorWebhookGenerateHmac, ConnectorWebhookRegister,
    },
    router_request_types::merchant_connector_webhook_management::{
        ConnectorWebhookGenerateHmacRequest, ConnectorWebhookRegisterRequest,
    },
    router_response_types::merchant_connector_webhook_management::{
        ConnectorWebhookGenerateHmacResponse, ConnectorWebhookRegisterResponse,
    },
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

/// trait WebhookGenerateHmac for V1
pub trait WebhookGenerateHmac:
    ConnectorIntegration<
    ConnectorWebhookGenerateHmac,
    ConnectorWebhookGenerateHmacRequest,
    ConnectorWebhookGenerateHmacResponse,
>
{
}

/// trait ConfigureConnectorWebhook for V1
pub trait ConfigureConnectorWebhook: ConnectorCommon + WebhookRegister {
    /// Whether this connector requires a separate HMAC generation call after registering the
    /// webhook. Connectors that override this to `true` MUST also implement
    /// [`GenerateConnectorWebhookHmac`].
    fn requires_webhook_hmac_generation(&self) -> bool {
        false
    }
}

/// trait GenerateConnectorWebhookHmac for V1
pub trait GenerateConnectorWebhookHmac: ConnectorCommon + WebhookGenerateHmac {}

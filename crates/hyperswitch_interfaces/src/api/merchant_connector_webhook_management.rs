//! Connector Webhooks Interface for V1
use hyperswitch_domain_models::{
    router_flow_types::merchant_connector_webhook_management::{
        ConnectorWebhookGenerateSecret, ConnectorWebhookRegister,
    },
    router_request_types::merchant_connector_webhook_management::{
        ConnectorWebhookGenerateSecretRequest, ConnectorWebhookRegisterRequest,
    },
    router_response_types::merchant_connector_webhook_management::{
        ConnectorWebhookGenerateSecretResponse, ConnectorWebhookRegisterResponse,
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
    /// Whether this connector requires a separate HMAC generation call after registering the
    /// webhook. Connectors that override this to `true` MUST also implement
    /// [`GenerateConnectorWebhookSecret`].
    fn requires_webhook_secret_generation(&self) -> bool {
        false
    }
}

/// trait WebhookGenerateSecret for V1
pub trait WebhookGenerateSecret:
    ConnectorIntegration<
    ConnectorWebhookGenerateSecret,
    ConnectorWebhookGenerateSecretRequest,
    ConnectorWebhookGenerateSecretResponse,
>
{
}

/// trait GenerateConnectorWebhookSecret for V1
pub trait GenerateConnectorWebhookSecret: ConnectorCommon + WebhookGenerateSecret {}

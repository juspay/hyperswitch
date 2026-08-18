//! Connector Webhooks Interface for V2
use hyperswitch_domain_models::{
    router_data_v2::flow_common_types::ConnectorWebhookConfigurationFlowData,
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

use crate::api::ConnectorIntegrationV2;

/// trait WebhookRegisterV2
pub trait WebhookRegisterV2:
    ConnectorIntegrationV2<
    ConnectorWebhookRegister,
    ConnectorWebhookConfigurationFlowData,
    ConnectorWebhookRegisterRequest,
    ConnectorWebhookRegisterResponse,
>
{
    /// Whether this connector requires a separate HMAC generation call after registering the
    /// webhook.
    fn requires_webhook_secret_generation(&self) -> bool {
        false
    }
}

/// trait WebhookGenerateSecretV2
pub trait WebhookGenerateSecretV2:
    ConnectorIntegrationV2<
    ConnectorWebhookGenerateSecret,
    ConnectorWebhookConfigurationFlowData,
    ConnectorWebhookGenerateSecretRequest,
    ConnectorWebhookGenerateSecretResponse,
>
{
}

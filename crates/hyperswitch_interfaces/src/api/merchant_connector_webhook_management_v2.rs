//! Connector Webhooks Interface for V2
use hyperswitch_domain_models::{
    router_data_v2::flow_common_types::ConnectorWebhookConfigurationFlowData,
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
}

/// trait WebhookGenerateHmacV2
pub trait WebhookGenerateHmacV2:
    ConnectorIntegrationV2<
    ConnectorWebhookGenerateHmac,
    ConnectorWebhookConfigurationFlowData,
    ConnectorWebhookGenerateHmacRequest,
    ConnectorWebhookGenerateHmacResponse,
>
{
}

/// trait ConfigureConnectorWebhook for V2
pub trait ConfigureConnectorWebhookV2:
    super::ConnectorCommon + WebhookRegisterV2 + WebhookGenerateHmacV2
{
    /// Whether this connector requires a separate HMAC generation call after registering the
    /// webhook.
    fn requires_webhook_hmac_generation(&self) -> bool {
        false
    }
}

//! Connector Webhooks Interface for V2

use hyperswitch_domain_models::{
    router_data_v2::flow_common_types::ConnectorWebhookConfigurationFlowData,
    router_flow_types::configure_connector_webhook::ConnectorWebhookRegister,
    router_request_types::configure_connector_webhook::ConnectorWebhookRegisterData,
    router_response_types::configure_connector_webhook::ConnectorWebhookRegisterResponse,
};

use crate::api::ConnectorIntegrationV2;

/// trait WebhookRegisterV2
pub trait WebhookRegisterV2:
    ConnectorIntegrationV2<
    ConnectorWebhookRegister,
    ConnectorWebhookConfigurationFlowData,
    ConnectorWebhookRegisterData,
    ConnectorWebhookRegisterResponse,
>
{
}

/// trait ConfigureConnectorWebhook for V2
pub trait ConfigureConnectorWebhookV2: super::ConnectorCommon + WebhookRegisterV2 {}

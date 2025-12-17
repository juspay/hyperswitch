#[cfg(feature = "v1")]
pub use api_models::admin::{
    ConnectorWebhookListResponse, ConnectorWebhookRegisterRequest, RegisterConnectorWebhookResponse,
};
pub use hyperswitch_domain_models::router_flow_types::configure_connector_webhook::ConnectorWebhookRegister;

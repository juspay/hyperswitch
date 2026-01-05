
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Register a webhook at the connector
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ConnectorWebhookRegisterRequest {
    #[schema(value_type = Option<ConnectorWebhookEventType>)]
    pub event_type: common_enums::ConnectorWebhookEventType,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RegisterConnectorWebhookResponse {
    #[schema(value_type = Option<ConnectorWebhookEventType>)]
    pub event_type: common_enums::ConnectorWebhookEventType,
    pub connector_webhook_id: Option<String>,
    #[schema(value_type = Option<WebhookRegistrationStatus>)]
    pub webhook_registration_status: common_enums::WebhookRegistrationStatus,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ConnectorWebhookListResponse {
    pub connector: String,
    pub webhooks: Vec<ConnectorWebhookResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ConnectorWebhookResponse {
    #[schema(value_type = Option<ConnectorWebhookEventType>)]
    pub event_type: common_enums::ConnectorWebhookEventType,
    pub connector_webhook_id: String,
}

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
    /// Status of the HMAC key generation. `None` when the connector does not require a
    /// separate HMAC generation step after registration.
    #[schema(value_type = Option<WebhookHmacGenerationStatus>)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hmac_generation_status: Option<common_enums::WebhookHmacGenerationStatus>,
    /// Connector error code when the HMAC generation step fails.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hmac_error_code: Option<String>,
    /// Connector error message when the HMAC generation step fails.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hmac_error_message: Option<String>,
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

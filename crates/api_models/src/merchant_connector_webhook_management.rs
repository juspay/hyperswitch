use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Register a webhook at the connector
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ConnectorWebhookRegisterRequest {
    #[schema(value_type = Option<ConnectorWebhookEventType>)]
    pub event_type: common_enums::ConnectorWebhookEventType,
}

/// Connector-reported error code and message for a single registration step.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ConnectorErrorDetails {
    pub code: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RegisterConnectorWebhookResponse {
    #[schema(value_type = Option<ConnectorWebhookEventType>)]
    pub event_type: common_enums::ConnectorWebhookEventType,
    pub connector_webhook_id: Option<String>,
    #[schema(value_type = Option<WebhookRegistrationStatus>)]
    pub webhook_registration_status: common_enums::WebhookRegistrationStatus,
    /// Connector error reported during webhook registration. `None` when registration succeeded.
    pub connector_error: Option<ConnectorErrorDetails>,
    /// Status of the HMAC key generation. `None` when the connector does not require a
    /// separate HMAC generation step after registration.
    #[schema(value_type = Option<WebhookSecretGenerationStatus>)]
    pub secret_generation_status: Option<common_enums::WebhookSecretGenerationStatus>,
    /// Connector error reported during the HMAC generation step. `None` when HMAC generation
    /// wasn't attempted or succeeded.
    pub secret_error: Option<ConnectorErrorDetails>,
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

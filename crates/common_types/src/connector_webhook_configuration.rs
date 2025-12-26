use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, ToSchema)]
/// Connector details for webhook configuration via hyperswitch API
pub struct WebhookSetupCapabilities {
    /// Indicates if the connector supports webhooks configuration via API
    pub is_webhook_auto_configuration_supported: bool,

    /// Indicates whether a webhook secret must be collected from the merchant for verification
    pub requires_webhook_secret: Option<bool>,

    /// The type of webhook configuration supported by the connector
    pub config_type: Option<WebhookConfigType>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, ToSchema)]
/// Enum to represent the type of webhook configuration
pub enum WebhookConfigType {
    /// Standard webhook configuration supporting all events hyperswitch provides
    Standard,
    /// Custom webhook configuration supporting only specific events
    #[schema(value_type = Option<EventType>)]
    CustomEvents(Vec<common_enums::EventType>),
}

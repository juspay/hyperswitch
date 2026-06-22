#[derive(serde::Serialize, Debug, Clone)]
pub struct ConnectorWebhookRegisterRequest {
    /// The scope of this webhook registration.
    pub scope: api_models::merchant_connector_webhook_management::ScopeIdentifier,
    /// The webhook URL to register.
    pub webhook_url: hyperswitch_masking::Secret<String>,
    /// The entire URL of the connector
    pub base_url: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConnectorWebhookData {
    pub event_type: common_enums::ConnectorWebhookEventType,
}

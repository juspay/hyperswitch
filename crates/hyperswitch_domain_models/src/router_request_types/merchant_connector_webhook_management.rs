#[derive(Debug, Clone)]
pub struct ConnectorWebhookRegisterRequest {
    pub event_type: common_enums::ConnectorWebhookEventType,
    pub webhook_url: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConnectorWebhookData {
    pub event_type: common_enums::ConnectorWebhookEventType,
}

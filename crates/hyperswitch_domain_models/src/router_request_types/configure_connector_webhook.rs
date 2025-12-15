#[derive(Debug, Clone)]
pub struct ConnectorWebhookRegisterData {
    pub event_type: common_enums::ConnectorWebhookEventType,
    pub webhook_url: String,
}
#[derive(Debug, Clone)]
pub struct ConnectorWebhookRegisterResponse {
    pub connector_webhook_id: Option<String>,
    pub status: common_enums::WebhookRegistrationStatus,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
}
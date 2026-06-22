#[derive(Debug, Clone)]
pub struct ConnectorWebhookRegisterResponse {
    /// The scope identifier this response is for.
    pub identifier: api_models::merchant_connector_webhook_management::ScopeIdentifier,
    /// Status of the registration.
    pub status: common_enums::WebhookRegistrationStatus,
    /// Connector-generated webhook ID, if successful.
    pub connector_webhook_id: Option<String>,
    /// Error code, if the registration failed.
    pub error_code: Option<String>,
    /// Error message, if the registration failed.
    pub error_message: Option<String>,
}

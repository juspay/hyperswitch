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
    /// Non-sensitive metadata to expose in the per-scope API response.
    pub response_metadata: Option<serde_json::Value>,
    /// Connector metadata patch to persist on the merchant connector account.
    pub metadata: Option<common_utils::pii::SecretSerdeValue>,
}

#[derive(Debug, Clone)]
pub struct ConnectorWebhookGenerateSecretResponse {
    pub secret: Option<hyperswitch_masking::Secret<String>>,
    pub status: common_enums::WebhookSecretGenerationStatus,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
}

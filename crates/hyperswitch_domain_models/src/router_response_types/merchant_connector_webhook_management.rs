#[derive(Debug, Clone, serde::Serialize)]
pub struct ConnectorWebhookRegisterResponse {
    /// The scope identifier this response is for.
    pub identifier:
        crate::router_request_types::merchant_connector_webhook_management::ScopeIdentifier,
    /// Status of the registration.
    pub status: common_enums::WebhookRegistrationStatus,
    /// Connector-generated webhook ID, if successful.
    pub connector_webhook_id: Option<String>,
    /// Connector-generated webhook secret returned during registration, if any.
    pub connector_webhook_secret: Option<hyperswitch_masking::Secret<String>>,
    /// Error code, if the registration failed.
    pub error_code: Option<String>,
    /// Error message, if the registration failed.
    pub error_message: Option<String>,
    pub metadata: Option<common_utils::pii::SecretSerdeValue>,
}

#[derive(Debug, Clone)]
pub struct ConnectorWebhookGenerateSecretResponse {
    pub secret: Option<hyperswitch_masking::Secret<String>>,
    pub status: common_enums::WebhookSecretGenerationStatus,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
}

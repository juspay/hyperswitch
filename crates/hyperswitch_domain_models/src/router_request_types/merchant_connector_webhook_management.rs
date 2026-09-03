#[derive(serde::Serialize, Debug, Clone)]
pub struct ConnectorWebhookRegisterRequest {
    /// The scope of this webhook registration.
    pub scope: api_models::merchant_connector_webhook_management::ScopeIdentifier,
    /// The webhook URL to register.
    pub webhook_url: hyperswitch_masking::Secret<url::Url>,
    /// The entire URL of the connector
    pub base_url: url::Url,
}

#[derive(Debug, Clone)]
pub struct ConnectorWebhookGenerateSecretRequest {
    pub connector_webhook_id: String,
}

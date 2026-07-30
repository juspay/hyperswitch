#[derive(serde::Serialize, Debug, Clone, PartialEq)]
pub enum ScopeIdentifier {
    NotSpecific,
    PaymentMethodType(common_enums::PaymentMethodType),
    EventType(common_enums::EventType),
    EventTypes(Vec<common_enums::EventType>),
}

#[derive(serde::Serialize, Debug, Clone)]
pub struct ConnectorWebhookRegisterRequest {
    /// The scope of this webhook registration.
    pub scope: ScopeIdentifier,
    /// The webhook URL to register.
    pub webhook_url: hyperswitch_masking::Secret<url::Url>,
    /// The entire URL of the connector
    pub base_url: url::Url,
}

#[derive(Debug, Clone)]
pub struct ConnectorWebhookGenerateSecretRequest {
    pub connector_webhook_id: String,
}

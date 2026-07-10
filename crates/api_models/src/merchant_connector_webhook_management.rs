use serde::{Deserialize, Deserializer, Serialize};
use utoipa::ToSchema;

use crate::enums::{
    ConnectorWebhookEventType, EventType, PaymentMethodType, WebhookRegistrationStatus,
};

/// The scope of webhook registration.
/// Determines which entities the connector should register webhooks for.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", content = "values", rename_all = "snake_case")]
#[non_exhaustive]
pub enum Scope {
    /// Connector does not scope webhooks to any specific entity
    /// Single registration call
    NotSpecific,
    /// Scoped by payment method types (e.g., Pix, Boleto)
    PaymentMethodTypes(Vec<PaymentMethodType>),
    /// Scoped by event types (e.g., Payments, Refunds, Disputes)
    EventTypes(Vec<EventType>),
}

/// Discriminator for the scope type in the response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ScopeType {
    NotSpecific,
    PaymentMethodType,
    EventType,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(into = "String", try_from = "String")]
pub enum ScopeIdentifier {
    NotSpecific,
    PaymentMethodType(PaymentMethodType),
    EventType(EventType),
}

impl From<ScopeIdentifier> for String {
    fn from(identifier: ScopeIdentifier) -> Self {
        match identifier {
            ScopeIdentifier::NotSpecific => "not_specific".to_owned(),
            ScopeIdentifier::PaymentMethodType(payment_method_type) => {
                payment_method_type.to_string()
            }
            ScopeIdentifier::EventType(event_type) => event_type.to_string(),
        }
    }
}

impl TryFrom<String> for ScopeIdentifier {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value == "not_specific" {
            return Ok(Self::NotSpecific);
        }
        if let Ok(payment_method_type) = value.parse::<PaymentMethodType>() {
            return Ok(Self::PaymentMethodType(payment_method_type));
        }
        if let Ok(event_type) = value.parse::<EventType>() {
            return Ok(Self::EventType(event_type));
        }
        Err(format!("unknown ScopeIdentifier: {value}"))
    }
}

/// Result of registering a webhook for a single scope identifier.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WebhookRegistrationResult {
    /// The scope identifier this result corresponds to.
    pub identifier: ScopeIdentifier,
    /// Whether the registration succeeded or failed.
    pub status: WebhookRegistrationStatus,
    /// The connector-generated webhook ID, if successful.
    pub connector_webhook_id: Option<String>,
    /// Error details, if the registration failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<WebhookRegistrationError>,
}

/// Error details for a failed webhook registration.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WebhookRegistrationError {
    pub code: String,
    pub message: String,
}

/// Register a webhook at the connector
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ConnectorWebhookRegisterRequest {
    #[schema(value_type = Option<Scope>)]
    pub scope: Option<Scope>,
    #[schema(value_type = Option<ConnectorWebhookEventType>, deprecated)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_type: Option<ConnectorWebhookEventType>,
}

fn event_type_to_scope(event_type: ConnectorWebhookEventType) -> Scope {
    match event_type {
        ConnectorWebhookEventType::AllEvents => Scope::NotSpecific,
        ConnectorWebhookEventType::SpecificEvent(event) => Scope::EventTypes(vec![event]),
    }
}

impl<'de> Deserialize<'de> for ConnectorWebhookRegisterRequest {
    #[allow(deprecated)]
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawConnectorWebhookRegisterRequest {
            scope: Option<Scope>,
            #[serde(default)]
            event_type: Option<ConnectorWebhookEventType>,
        }

        let raw = RawConnectorWebhookRegisterRequest::deserialize(deserializer)?;

        let scope = match (raw.scope, raw.event_type) {
            (Some(scope), _) => scope,
            (None, Some(event_type)) => event_type_to_scope(event_type),
            (None, None) => {
                return Err(serde::de::Error::custom(
                    "missing field: either `scope` or `event_type` must be provided",
                ))
            }
        };

        Ok(Self {
            scope: Some(scope),
            event_type: raw.event_type,
        })
    }
}

/// Connector-reported error code and message from the webhook secret generation step.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct WebhookSecretErrorDetails {
    pub code: Option<String>,
    pub message: Option<String>,
}

/// Response for registering connector webhooks.
/// This struct combines the legacy fields with the new scope-based fields
/// response remains backward-compatible. Legacy clients can continue reading the original fields
/// New clients can use `scope_type`, `requested`, and `results`.
#[allow(deprecated)]
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RegisterConnectorWebhookResponse {
    /// To be Deprecated soon; prefer the scope-based response fields (`scope_type`, `requested`, `results`).
    #[schema(value_type = Option<ConnectorWebhookEventType>, deprecated)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_type: Option<ConnectorWebhookEventType>,
    /// To be Deprecated soon; prefer `results` for per-identifier registration outcomes.
    #[schema(value_type = Option<String>)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connector_webhook_id: Option<String>,
    /// To be Deprecated soon; prefer `results` for per-identifier registration outcomes.
    #[schema(value_type = Option<WebhookRegistrationStatus>)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub webhook_registration_status: Option<WebhookRegistrationStatus>,
    /// To be Deprecated soon; prefer `results` for per-identifier registration outcomes.
    #[schema(value_type = Option<String>)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    /// To be Deprecated soon; prefer `results` for per-identifier registration outcomes.
    #[schema(value_type = Option<String>)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    /// Status of the webhook secret key generation. `None` when the connector does not require a separate webhook secret generation step after registration.
    #[schema(value_type = Option<WebhookSecretGenerationStatus>)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret_generation_status: Option<common_enums::WebhookSecretGenerationStatus>,
    /// Connector error reported during the HMAC generation step. `None` when HMAC generation wasn't attempted or succeeded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret_error: Option<WebhookSecretErrorDetails>,
    /// The type of scope used for this registration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope_type: Option<ScopeType>,
    /// List of identifiers that were requested to be registered.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requested: Option<Vec<ScopeIdentifier>>,
    /// Per-identifier registration results.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub results: Option<Vec<WebhookRegistrationResult>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ConnectorWebhookListResponse {
    pub connector: String,
    pub webhooks: Vec<ConnectorWebhookResponse>,
}

/// Scope of a single registered connector webhook.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ConnectorWebhookScope {
    NotSpecific,
    PaymentMethodType { value: PaymentMethodType },
    EventType { value: EventType },
}

#[allow(deprecated)]
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LegacyConnectorWebhookResponse {
    #[schema(value_type = Option<ConnectorWebhookEventType>, deprecated)]
    pub event_type: Option<ConnectorWebhookEventType>,
    pub connector_webhook_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ScopeBasedConnectorWebhookResponse {
    pub connector_webhook_id: String,
    pub scope: ConnectorWebhookScope,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(untagged)]
pub enum ConnectorWebhookResponse {
    Legacy(LegacyConnectorWebhookResponse),
    ScopeBased(ScopeBasedConnectorWebhookResponse),
}

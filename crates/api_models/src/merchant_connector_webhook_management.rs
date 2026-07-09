use serde::{Deserialize, Deserializer, Serialize};
use utoipa::ToSchema;

use crate::enums::{EventType, PaymentMethodType, WebhookRegistrationStatus};

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
    pub event_type: Option<common_enums::ConnectorWebhookEventType>,
    /// Internal marker set during deserialization when the caller used the deprecated
    /// `event_type` field instead of `scope`. Used to decide whether to emit legacy response
    /// fields for backward compatibility.
    #[serde(skip)]
    pub is_legacy_request: bool,
}

impl ConnectorWebhookRegisterRequest {
    pub fn is_legacy_request(&self) -> bool {
        self.is_legacy_request
    }
}

fn event_type_to_scope(event_type: common_enums::ConnectorWebhookEventType) -> Scope {
    match event_type {
        common_enums::ConnectorWebhookEventType::AllEvents => Scope::NotSpecific,
        common_enums::ConnectorWebhookEventType::SpecificEvent(event) => {
            Scope::EventTypes(vec![event])
        }
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
            event_type: Option<common_enums::ConnectorWebhookEventType>,
        }

        let raw = RawConnectorWebhookRegisterRequest::deserialize(deserializer)?;

        let (scope, is_legacy_request) = match (raw.scope, raw.event_type) {
            (Some(scope), _) => (scope, false),
            (None, Some(event_type)) => (event_type_to_scope(event_type), true),
            (None, None) => {
                return Err(serde::de::Error::custom(
                    "missing field: either `scope` or deprecated `event_type` must be provided",
                ))
            }
        };

        Ok(Self {
            scope: Some(scope),
            event_type: raw.event_type,
            is_legacy_request,
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

#[allow(deprecated)]
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LegacyRegisterConnectorWebhookResponse {
    #[schema(value_type = Option<ConnectorWebhookEventType>, deprecated)]
    pub event_type: Option<common_enums::ConnectorWebhookEventType>,
    #[schema(value_type = Option<String>, deprecated)]
    pub connector_webhook_id: Option<String>,
    #[schema(value_type = Option<WebhookRegistrationStatus>, deprecated)]
    pub webhook_registration_status: Option<WebhookRegistrationStatus>,
    #[schema(value_type = Option<String>, deprecated)]
    pub error_code: Option<String>,
    #[schema(value_type = Option<String>, deprecated)]
    pub error_message: Option<String>,
    #[schema(value_type = Option<WebhookSecretGenerationStatus>)]
    pub secret_generation_status: Option<common_enums::WebhookSecretGenerationStatus>,
    pub secret_error: Option<WebhookSecretErrorDetails>,
}

/// Response for registering connector webhooks using the new scope-based model.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ScopeBasedRegisterConnectorWebhookResponse {
    /// The type of scope used for this registration.
    pub scope_type: ScopeType,
    /// List of identifiers that were requested to be registered.
    pub requested: Vec<ScopeIdentifier>,
    /// Per-identifier registration results.
    pub results: Vec<WebhookRegistrationResult>,
    /// Status of the webhook secret key generation. `None` when the connector does not require a
    /// separate webhook secret generation step after registration.
    #[schema(value_type = Option<WebhookSecretGenerationStatus>)]
    pub secret_generation_status: Option<common_enums::WebhookSecretGenerationStatus>,
    /// Connector error reported during the HMAC generation step. `None` when HMAC generation
    /// wasn't attempted or succeeded.
    pub secret_error: Option<WebhookSecretErrorDetails>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(untagged)]
pub enum RegisterConnectorWebhookResponse {
    Legacy(LegacyRegisterConnectorWebhookResponse),
    ScopeBased(ScopeBasedRegisterConnectorWebhookResponse),
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
    pub event_type: Option<common_enums::ConnectorWebhookEventType>,
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

use std::fmt;

use serde::{de::Visitor, Deserialize, Deserializer, Serialize, Serializer};
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

#[derive(Debug, Clone, ToSchema)]
pub enum ScopeIdentifier {
    NotSpecific,
    PaymentMethodType(PaymentMethodType),
    EventType(EventType),
}

impl Serialize for ScopeIdentifier {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::NotSpecific => serializer.serialize_str("not_specific"),
            Self::PaymentMethodType(v) => v.serialize(serializer),
            Self::EventType(v) => v.serialize(serializer),
        }
    }
}

struct ScopeIdentifierVisitor;

impl Visitor<'_> for ScopeIdentifierVisitor {
    type Value = ScopeIdentifier;

    /// Provides a description of the expected input format for deserialization error messages.
    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .write_str("\"not_specific\", a payment method type string, or an event type string")
    }

    /// Handles null/JSON unit values by defaulting to NotSpecific.
    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(ScopeIdentifier::NotSpecific)
    }

    /// Handles explicit None values by defaulting to NotSpecific.
    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(ScopeIdentifier::NotSpecific)
    }

    /// Parses a JSON string into ScopeIdentifier by trying each variant in order:
    /// "not_specific" literal
    /// Parse as PaymentMethodType (e.g. "pix", "boleto")
    /// Parse as EventType (e.g. "payments", "refunds")
    /// Returns a deserialization error if none match.
    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        if v == "not_specific" {
            return Ok(ScopeIdentifier::NotSpecific);
        }
        if let Ok(pmt) = v.parse::<PaymentMethodType>() {
            return Ok(ScopeIdentifier::PaymentMethodType(pmt));
        }
        if let Ok(evt) = v.parse::<EventType>() {
            return Ok(ScopeIdentifier::EventType(evt));
        }
        Err(E::custom(format!("unknown ScopeIdentifier: {v}")))
    }
}

impl<'de> Deserialize<'de> for ScopeIdentifier {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(ScopeIdentifierVisitor)
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
    #[schema(value_type = Scope)]
    pub scope: Scope,
    #[deprecated(note = "Use `scope` instead to specify the event type for registration.")]
    #[schema(value_type = Option<ConnectorWebhookEventType>)]
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
            scope,
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

/// Response for registering connector webhooks.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RegisterConnectorWebhookResponse {
    // Deprecated fields retained for backward compatibility.
    #[schema(value_type = Option<ConnectorWebhookEventType>)]
    #[deprecated(note = "Use `scope_type` and `results` instead.")]
    pub event_type: Option<common_enums::ConnectorWebhookEventType>,
    #[schema(value_type = Option<String>)]
    #[deprecated(note = "Use `results` instead.")]
    pub connector_webhook_id: Option<String>,
    #[schema(value_type = Option<WebhookRegistrationStatus>)]
    #[deprecated(note = "Use `results` instead.")]
    pub webhook_registration_status: Option<WebhookRegistrationStatus>,
    #[schema(value_type = Option<String>)]
    #[deprecated(note = "Use `results` instead.")]
    pub error_code: Option<String>,
    #[schema(value_type = Option<String>)]
    #[deprecated(note = "Use `results` instead.")]
    pub error_message: Option<String>,
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

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ConnectorWebhookResponse {
    #[schema(value_type = Option<ConnectorWebhookEventType>)]
    #[deprecated(note = "Use `scope` instead.")]
    pub event_type: Option<common_enums::ConnectorWebhookEventType>,
    pub connector_webhook_id: String,
    pub scope: ConnectorWebhookScope,
}

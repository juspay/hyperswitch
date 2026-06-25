use std::fmt;

use serde::{de::Visitor, Deserialize, Deserializer, Serialize, Serializer};
use utoipa::ToSchema;

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
    PaymentMethodTypes(Vec<common_enums::PaymentMethodType>),
    /// Scoped by event types (e.g., Payments, Refunds, Disputes)
    EventTypes(Vec<common_enums::EventType>),
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
    PaymentMethodType(common_enums::PaymentMethodType),
    EventType(common_enums::EventType),
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
        if let Ok(pmt) = v.parse::<common_enums::PaymentMethodType>() {
            return Ok(ScopeIdentifier::PaymentMethodType(pmt));
        }
        if let Ok(evt) = v.parse::<common_enums::EventType>() {
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
    pub status: common_enums::WebhookRegistrationStatus,
    /// The connector-generated webhook ID, if successful.
    pub connector_webhook_id: Option<String>,
    /// Error details, if the registration failed.
    pub error: Option<WebhookRegistrationError>,
}

/// Error details for a failed webhook registration.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WebhookRegistrationError {
    pub code: String,
    pub message: String,
}

/// Register a webhook at the connector
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ConnectorWebhookRegisterRequest {
    #[schema(value_type = Option<Scope>)]
    pub scope: Scope,
    #[deprecated(note = "Use `scope` instead to specify the event type for registration.")]
    #[schema(value_type = Option<ConnectorWebhookEventType>)]
    pub event_type: Option<common_enums::ConnectorWebhookEventType>,
}

/// Response for registering connector webhooks.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RegisterConnectorWebhookResponse {
    /// The type of scope used for this registration.
    pub scope_type: ScopeType,
    /// List of identifiers that were requested to be registered.
    pub requested: Vec<ScopeIdentifier>,
    /// Per-identifier registration results.
    pub results: Vec<WebhookRegistrationResult>,
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
    PaymentMethodType {
        value: common_enums::PaymentMethodType,
    },
    EventType {
        value: common_enums::EventType,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ConnectorWebhookResponse {
    pub connector_webhook_id: String,
    pub scope: ConnectorWebhookScope,
}

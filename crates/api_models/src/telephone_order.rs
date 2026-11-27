pub mod twilio;

use std::collections::HashMap;

use utoipa::ToSchema;

/// The IVR's `<Parameter>` tags. The ones read by Hyperswitch are named below; the rest ride along
/// into the payment's `metadata`. Keys must be lowercase — Twilio lowercases parameter names, so a
/// camelCase one lands in the passthrough instead. A tag's value is always a string, so the
/// object-valued keys — `customer_acceptance` and `billing` — are read from a JSON string as well
/// as from an object.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct TelephoneOrderParameters {
    /// Charges this payment id rather than one derived from the provider's attempt id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<String>, example = "pay_1234")]
    pub payment_id: Option<common_utils::id_type::PaymentId>,

    /// The profile to charge against. Absent, the merchant's default profile is used.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<String>, example = "pro_1234")]
    pub profile_id: Option<common_utils::id_type::ProfileId>,

    /// The customer this payment belongs to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<String>, example = "cus_1234")]
    pub customer_id: Option<common_utils::id_type::CustomerId>,

    /// Who captures the authorization. `automatic` by default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<TelephoneOrderCaptureMethod>, example = "manual")]
    pub capture_method: Option<TelephoneOrderCaptureMethod>,

    /// Stores the collected card against the customer. Needs `customer_id` and
    /// `customer_acceptance` alongside it, and is never defaulted. `off_session` is the one that
    /// carries authority for later merchant-initiated charges; `on_session` does not.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<FutureUsage>, example = "off_session")]
    pub setup_future_usage: Option<common_enums::FutureUsage>,

    /// Currency for a `tokenize`, which carries no amount of its own. A charge ignores it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<Currency>, example = "USD")]
    pub currency: Option<common_enums::Currency>,

    /// The caller's agreement to their card being kept, taken on the call. **Required whenever a
    /// card is stored** — a `tokenize`, or a charge carrying `setup_future_usage`. Recorded as the
    /// stored-credential consent the networks require, so it says only what the IVR reported.
    /// Use `acceptance_type: offline`; `accepted_at` defaults to now.
    #[serde(
        default,
        deserialize_with = "object_or_json_string",
        skip_serializing_if = "Option::is_none"
    )]
    #[schema(value_type = Option<CustomerAcceptance>)]
    pub customer_acceptance: Option<common_types::payments::CustomerAcceptance>,

    // `skip_psp` drops Cybersource's `TOKEN_CREATE`, which is the way past an account without
    // Token Management Service, at the cost of the `connector_mandate_id`.
    /// Whether the connector should mint a token of its own for the stored card. Left unset,
    /// Hyperswitch decides. Currently read only by Cybersource.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<Tokenization>, example = "skip_psp")]
    pub tokenization: Option<common_enums::Tokenization>,

    // Cybersource wants `last_name`, `line1`, `city`, `country` and an email on a zero-amount
    // mandate, so a tokenize that skips this is refused at the connector, not here.
    /// The caller's billing address and email, in the same shape `/payments` takes. The provider's
    /// own postal code, where it collects one, wins over `address.zip`.
    #[serde(
        default,
        deserialize_with = "object_or_json_string",
        skip_serializing_if = "Option::is_none"
    )]
    #[schema(value_type = Option<Address>)]
    pub billing: Option<crate::payments::Address>,

    /// Whatever else the merchant put on the `<Pay>` verb, carried through unread.
    #[serde(flatten)]
    #[schema(value_type = Object, example = json!({"order_ref": "ORD-1234"}))]
    pub extra: HashMap<String, String>,
}

/// A `<Parameter>` tag's value is always a string, so an object-valued key reaches us as a JSON
/// string. A caller that posts structured parameters sends the object itself. Both are read.
fn object_or_json_string<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::de::DeserializeOwned,
{
    use serde::Deserialize;

    match Option::<serde_json::Value>::deserialize(deserializer)? {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::String(raw)) => serde_json::from_str(&raw)
            .map(Some)
            .map_err(serde::de::Error::custom),
        Some(value) => serde_json::from_value(value)
            .map(Some)
            .map_err(serde::de::Error::custom),
    }
}

// Narrower than `common_enums::CaptureMethod` on purpose: `Scheduled` needs a `capture_on` no
// `<Parameter>` can set, and most connectors reject it and `ManualMultiple` as not implemented.

/// Specifies how the authorization is captured.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TelephoneOrderCaptureMethod {
    /// Captured in full as soon as the authorization succeeds.
    Automatic,
    /// Authorized only; the merchant captures it themselves.
    Manual,
}

impl From<TelephoneOrderCaptureMethod> for common_enums::CaptureMethod {
    fn from(value: TelephoneOrderCaptureMethod) -> Self {
        match value {
            TelephoneOrderCaptureMethod::Automatic => Self::Automatic,
            TelephoneOrderCaptureMethod::Manual => Self::Manual,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The shape Twilio actually posts: every `<Parameter>` value is a string, so the
    /// object-valued keys arrive as escaped JSON.
    #[test]
    fn reads_object_parameters_from_json_strings() {
        let parameters: TelephoneOrderParameters = serde_json::from_value(serde_json::json!({
            "customer_acceptance": "{\"acceptance_type\":\"offline\"}",
            "profile_id": "pro_1234",
            "setup_future_usage": "off_session",
            "currency": "USD",
            "customer_id": "cus_1234",
            "billing": "{\"email\":\"caller@example.com\",\"address\":{\"first_name\":\"John\",\"last_name\":\"Doe\",\"line1\":\"1467 Harrison Street\",\"city\":\"San Francisco\",\"state\":\"CA\",\"country\":\"US\"}}"
        }))
        .expect("Twilio's string-valued parameters should deserialize");

        assert_eq!(
            parameters
                .customer_acceptance
                .expect("customer_acceptance should be read")
                .acceptance_type,
            common_types::payments::AcceptanceType::Offline
        );

        let billing = parameters.billing.expect("billing should be read");
        let address = billing.address.expect("billing address should be read");
        assert_eq!(address.city.as_deref(), Some("San Francisco"));
        assert_eq!(
            address.country,
            Some(common_enums::CountryAlpha2::US)
        );
    }

    /// The shape a caller posting structured parameters sends. Both forms reach the same payment.
    #[test]
    fn reads_object_parameters_from_objects() {
        let parameters: TelephoneOrderParameters = serde_json::from_value(serde_json::json!({
            "customer_acceptance": { "acceptance_type": "offline" },
            "billing": { "email": "caller@example.com" }
        }))
        .expect("structured parameters should deserialize");

        assert!(parameters.customer_acceptance.is_some());
        assert!(parameters.billing.is_some());
    }

    /// Keys the endpoint does not name ride along to the payment's metadata, string-valued.
    #[test]
    fn carries_unknown_parameters_through() {
        let parameters: TelephoneOrderParameters = serde_json::from_value(serde_json::json!({
            "customer_id": "cus_1234",
            "order_ref": "ORD-1234"
        }))
        .expect("unknown parameters should deserialize");

        assert_eq!(
            parameters.extra.get("order_ref").map(String::as_str),
            Some("ORD-1234")
        );
    }

    /// A malformed JSON string is rejected rather than dropped: an IVR that sends a broken
    /// acceptance must not look like one that sent none, which would store the card without it.
    #[test]
    fn rejects_a_malformed_json_string() {
        let error = serde_json::from_value::<TelephoneOrderParameters>(serde_json::json!({
            "customer_acceptance": "{\"acceptance_type\":"
        }))
        .expect_err("a malformed acceptance should not deserialize");

        assert!(error.to_string().contains("EOF"), "{error}");
    }
}

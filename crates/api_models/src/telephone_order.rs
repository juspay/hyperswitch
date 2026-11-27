pub mod twilio;

use std::collections::HashMap;

use utoipa::ToSchema;

/// Keys of the `parameters` map a response carries back, named where Hyperswitch reads them again
/// itself so a rename cannot take the id out of the API event.
pub mod response_parameters {
    /// The payment this call became. Carried on a tokenize and a failure, which have no
    /// `charge_id`.
    pub const PAYMENT_ID: &str = "payment_id";
    /// The card this call stored, where it stored one.
    pub const PAYMENT_METHOD_ID: &str = "payment_method_id";
}

/// The IVR's `<Parameter>` tags. The ones read by Hyperswitch are named below; the rest ride along
/// into the payment's `metadata`. Keys must be lowercase — Twilio lowercases parameter names, so a
/// camelCase one lands in the passthrough instead. A tag's value is always a string, so the
/// structured keys — `customer_acceptance` and `billing` — carry their JSON as text for the core
/// to parse.
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
    ///
    /// A `CustomerAcceptance` as JSON. Use `acceptance_type: offline` — there is no browser
    /// session to record an IP against — and `accepted_at` defaults to now.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(example = json!(r#"{"acceptance_type":"offline","accepted_at":"2026-11-26T10:11:12Z"}"#))]
    pub customer_acceptance: Option<String>,

    // `skip_psp` drops Cybersource's `TOKEN_CREATE`, which is the way past an account without
    // Token Management Service, at the cost of the `connector_mandate_id`.
    /// Whether the connector should mint a token of its own for the stored card. Left unset,
    /// Hyperswitch decides. Currently read only by Cybersource.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<Tokenization>, example = "skip_psp")]
    pub tokenization: Option<common_enums::Tokenization>,

    // Cybersource wants `last_name`, `line1`, `city`, `country` and an email on a zero-amount
    // mandate, so a tokenize that skips this is refused at the connector, not here.
    /// The caller's billing address and email, as an `Address` in JSON. The provider's own postal
    /// code, where it collects one, overrides `address.zip`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(example = json!(r#"{"email":"caller@example.com","address":{"last_name":"Doe","line1":"1467 Harrison Street","city":"San Francisco","state":"CA","country":"US"}}"#))]
    pub billing: Option<String>,

    /// Whatever else the merchant put on the `<Pay>` verb, carried through unread.
    #[serde(flatten)]
    #[schema(value_type = Object, example = json!({"order_ref": "ORD-1234"}))]
    pub extra: HashMap<String, String>,
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

    /// The shape Twilio actually posts: every `<Parameter>` value is a string, so the structured
    /// keys arrive as JSON text and are held as such.
    #[test]
    fn reads_structured_parameters_as_text() {
        let parameters: TelephoneOrderParameters = serde_json::from_value(serde_json::json!({
            "customer_acceptance": "{\"acceptance_type\":\"offline\"}",
            "profile_id": "pro_1234",
            "setup_future_usage": "off_session",
            "currency": "USD",
            "customer_id": "cus_1234",
            "billing": "{\"email\":\"caller@example.com\"}"
        }))
        .expect("Twilio's string-valued parameters should deserialize");

        assert_eq!(
            parameters.customer_acceptance.as_deref(),
            Some(r#"{"acceptance_type":"offline"}"#)
        );
        assert_eq!(
            parameters.billing.as_deref(),
            Some(r#"{"email":"caller@example.com"}"#)
        );
        assert_eq!(
            parameters.setup_future_usage,
            Some(common_enums::FutureUsage::OffSession)
        );
    }

    /// An object is refused rather than quietly accepted: Twilio cannot send one, so a caller
    /// that does has mistaken the shape.
    #[test]
    fn rejects_an_object() {
        let error = serde_json::from_value::<TelephoneOrderParameters>(serde_json::json!({
            "billing": { "email": "caller@example.com" }
        }))
        .expect_err("an object should not deserialize");

        assert!(error.to_string().contains("expected a string"), "{error}");
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
}

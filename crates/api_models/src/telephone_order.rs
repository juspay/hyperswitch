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

/// Codes shared by every provider. A provider adds its own for what it rejects on its way in.
pub mod error_codes {
    pub const INVALID_REQUEST: &str = "INVALID_REQUEST";
    pub const PROFILE_NOT_FOUND: &str = "PROFILE_NOT_FOUND";
    pub const PAYMENT_NOT_COMPLETED: &str = "PAYMENT_NOT_COMPLETED";
    pub const PROCESSING_ERROR: &str = "PROCESSING_ERROR";
    pub const DUPLICATE_TRANSACTION: &str = "DUPLICATE_TRANSACTION";
    /// The charge reached the connector with no final outcome. Kept apart from a decline so an
    /// IVR can be told not to ask for another card: the first authorization may be standing.
    pub const PAYMENT_PENDING: &str = "PAYMENT_PENDING";
    /// A card cannot be stored against nobody. Named apart so the IVR fix is unambiguous.
    pub const CUSTOMER_REQUIRED: &str = "CUSTOMER_REQUIRED";
    /// A card cannot be stored without consent, and the IVR has to ask before it can send one.
    pub const ACCEPTANCE_REQUIRED: &str = "ACCEPTANCE_REQUIRED";
    /// Verified card, nothing stored. Not a decline — another card would fail the same way.
    pub const TOKENIZE_FAILED: &str = "TOKENIZE_FAILED";
}

/// The IVR's `<Parameter>` tags. The ones read by Hyperswitch are named below; the rest ride along
/// into the payment's `metadata`. Keys must be lowercase — Twilio lowercases parameter names, so a
/// camelCase one lands in the passthrough instead. A tag's value is always a string, so the
/// structured keys — `customer_acceptance` and `billing` — carry their JSON as text, which
/// `JsonString` reads back into the type.
#[serde_with::serde_as]
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
    #[serde_as(as = "Option<serde_with::json::JsonString>")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(
        value_type = Option<String>,
        example = json!(r#"{"acceptance_type":"offline","accepted_at":"2026-11-26T10:11:12Z"}"#)
    )]
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
    /// The caller's billing address and email, as an `Address` in JSON. The provider's own postal
    /// code, where it collects one, overrides `address.zip`.
    #[serde_as(as = "Option<serde_with::json::JsonString>")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(
        value_type = Option<String>,
        example = json!(r#"{"email":"caller@example.com","address":{"last_name":"Doe","line1":"1467 Harrison Street","city":"San Francisco","state":"CA","country":"US"}}"#)
    )]
    pub billing: Option<crate::payments::Address>,

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

/// Why a charge failed, in terms the provider can read back to the caller.
#[derive(Debug)]
pub struct Failure {
    pub code: String,
    pub message: String,
    pub parameters: ResponseParameters,
}

impl Failure {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            parameters: ResponseParameters::new(),
        }
    }

    /// Only for a failure with a payment behind it; one rejected earlier has nothing to point at.
    pub fn with_parameters(mut self, parameters: ResponseParameters) -> Self {
        self.parameters = parameters;
        self
    }

    /// A failure of ours: reported without detail, investigated from the logs.
    pub fn processing_error() -> Self {
        Self::new(
            error_codes::PROCESSING_ERROR,
            "Payment could not be processed",
        )
    }
}

pub type ResponseParameters = HashMap<String, String>;

// Mapping a request onto a `PaymentsRequest`, and the endpoint that does it, are v1 only.

/// The half of the payment every provider fills in the same way, read out of
/// [`TelephoneOrderParameters`].
#[cfg(feature = "v1")]
pub struct Context {
    /// `None` falls back to the merchant's default profile.
    pub profile_id: Option<common_utils::id_type::ProfileId>,
    /// What makes a provider's retry idempotent: it collides on `(payment_id, merchant_id)`
    /// instead of charging again. Settled by the adapter's `resolve_payment_id`.
    pub payment_id: Option<common_utils::id_type::PaymentId>,
    pub customer_id: Option<common_utils::id_type::CustomerId>,
    /// `None` auto-captures. `Manual` leaves the capture to the merchant.
    pub capture_method: Option<common_enums::CaptureMethod>,
    /// `Some` keeps the card against the customer. Never defaulted — unattended charging is
    /// authority the merchant claims, not something we assume.
    pub setup_future_usage: Option<common_enums::FutureUsage>,
    /// The caller's agreement to that, as the IVR reported it. Never synthesised.
    pub customer_acceptance: Option<common_types::payments::CustomerAcceptance>,
    /// Whether the connector mints its own token. `None` leaves it to Hyperswitch; see
    /// [`TelephoneOrderParameters::tokenization`].
    pub tokenization: Option<common_enums::Tokenization>,
    /// What the IVR knows about the caller. Merged with the provider's postal code by
    /// [`Context::billing_address`].
    pub billing: Option<crate::payments::Address>,
    pub metadata: Option<serde_json::Value>,
}

#[cfg(feature = "v1")]
impl Context {
    /// The IVR's address, with the postal code the provider collected written over its `zip`. The
    /// caller keyed that in on this call, so it beats a `<Parameter>` address built from whatever
    /// the merchant's records hold.
    pub fn billing_address(
        &self,
        postal_code: Option<hyperswitch_masking::Secret<String>>,
    ) -> Option<crate::payments::Address> {
        if self.billing.is_none() && postal_code.is_none() {
            return None;
        }

        let mut billing = self.billing.clone().unwrap_or_default();
        // Only when there is one: assigning `None` would clear a zip the IVR sent.
        if let Some(postal_code) = postal_code {
            billing.address.get_or_insert_with(Default::default).zip = Some(postal_code);
        }
        Some(billing)
    }

    /// The shared fields, for a provider to fill the rest in with `..context.payments_request()`.
    /// Confirmed, flagged a telephone order, never 3DS — a caller has no browser. Auto-captured
    /// unless the IVR asked otherwise.
    pub fn payments_request(self) -> crate::payments::PaymentsRequest {
        crate::payments::PaymentsRequest {
            confirm: Some(true),
            capture_method: Some(
                self.capture_method
                    .unwrap_or(common_enums::CaptureMethod::Automatic),
            ),
            authentication_type: Some(common_enums::AuthenticationType::NoThreeDs),
            payment_channel: Some(common_enums::PaymentChannel::TelephoneOrder),
            profile_id: self.profile_id,
            payment_id: self
                .payment_id
                .map(crate::payments::PaymentIdType::PaymentIntentId),
            customer_id: self.customer_id,
            setup_future_usage: self.setup_future_usage,
            customer_acceptance: self.customer_acceptance,
            tokenization: self.tokenization,
            // Also on the billing address, which is where connectors look first.
            email: self
                .billing
                .as_ref()
                .and_then(|billing| billing.email.clone()),
            metadata: self.metadata,
            // `connector` is left unset so the merchant's configured routing decides.
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    // `Email` derefs to a `Secret`, so reading one back needs this in scope.
    use hyperswitch_masking::PeekInterface;

    use super::*;

    /// The shape Twilio actually posts: every `<Parameter>` value is a string, so the structured
    /// keys arrive as JSON text and are read back into their types.
    #[test]
    fn reads_structured_parameters_from_json_text() {
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
            parameters
                .customer_acceptance
                .expect("customer_acceptance should be read")
                .acceptance_type,
            common_types::payments::AcceptanceType::Offline
        );
        assert_eq!(
            parameters
                .billing
                .and_then(|billing| billing.email)
                .map(|email| email.peek().to_owned())
                .as_deref(),
            Some("caller@example.com")
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

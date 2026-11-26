pub mod twilio;

use std::collections::HashMap;

use utoipa::ToSchema;

/// The IVR's `<Parameter>` tags. The ones read by Hyperswitch are named below; the rest ride along
/// into the payment's `metadata`. Keys must be lowercase — Twilio lowercases parameter names, so a
/// camelCase one lands in the passthrough instead.
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<CustomerAcceptance>)]
    pub customer_acceptance: Option<common_types::payments::CustomerAcceptance>,

    // `skip_psp` drops Cybersource's `TOKEN_CREATE`, which is the way past an account without
    // Token Management Service, at the cost of the `connector_mandate_id`.
    /// Whether the connector should mint a token of its own for the stored card. Left unset,
    /// Hyperswitch decides. Currently read only by Cybersource.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<Tokenization>, example = "skip_psp")]
    pub tokenization: Option<common_enums::Tokenization>,

    // Twilio cannot populate this — `<Parameter>` carries flat string pairs — but Cybersource
    // wants `last_name`, `line1`, `city`, `country` and an email on a zero-amount mandate.
    /// The caller's billing address and email, in the same shape `/payments` takes. The provider's
    /// own postal code, where it collects one, wins over `address.zip`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<Address>)]
    pub billing: Option<crate::payments::Address>,

    /// Whatever else the merchant put on the `<Pay>` verb, carried through unread.
    #[serde(flatten)]
    #[schema(value_type = Object, example = json!({"order_ref": "ORD-99812"}))]
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

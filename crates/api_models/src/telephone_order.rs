pub mod twilio;

use std::collections::HashMap;

use utoipa::ToSchema;

/// The IVR's `<Parameter>` tags. The four read by Hyperswitch are named below; the rest ride along
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

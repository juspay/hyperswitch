pub mod twilio;

use std::collections::HashMap;

use utoipa::ToSchema;

/// The IVR's `<Parameter>` tags. The four Hyperswitch reads are named; the rest are the merchant's
/// and ride along into the payment's `metadata`. Keys are lowercase: Twilio lowercases parameter
/// names, so a camelCase one never matches and lands in the passthrough instead.
///
/// Values stay `String` so a bad one fails the charge with an `error_code` in a `200`, rather than
/// failing deserialization and answering with an HTTP error the provider cannot read.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct TelephoneOrderParameters {
    /// Charges this payment id rather than one derived from the provider's attempt id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(example = "pay_8320b449b06630271e378214dab80ce5")]
    pub payment_id: Option<String>,

    /// The profile to charge against. Absent, the merchant's default profile is used.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(example = "pro_GUMsmmzTU7oadT00pJeo")]
    pub profile_id: Option<String>,

    /// An unusable value is dropped, not rejected.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(example = "cus_1a2b3c4d5e6f7g8h9i0j")]
    pub customer_id: Option<String>,

    /// `automatic` (the default) or `manual`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(example = "manual")]
    pub capture_method: Option<String>,

    /// Whatever else the merchant put on the `<Pay>` verb, carried through unread.
    #[serde(flatten)]
    #[schema(value_type = Object, example = json!({"order_ref": "ORD-99812"}))]
    pub extra: HashMap<String, String>,
}

impl TelephoneOrderParameters {
    pub fn is_empty(&self) -> bool {
        self.extra.is_empty()
            && self.payment_id.is_none()
            && self.profile_id.is_none()
            && self.customer_id.is_none()
            && self.capture_method.is_none()
    }
}

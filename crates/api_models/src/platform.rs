use common_enums::MerchantAccountType;
use utoipa::ToSchema;

/// Represents the initiator context in platform-connected setups
/// Used in payment/refund/dispute responses to indicate who initiated the operation
/// None indicates a standard merchant flow / JWT flow / Admin flow or insufficient information
#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    PartialEq,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    strum::EnumString,
    ToSchema,
)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum Initiator {
    /// Platform merchant initiated the operation on behalf of connected merchant
    Platform,
    /// Connected merchant initiated the operation directly in a platform setup
    Connected,
}

impl Initiator {
    /// Converts MerchantAccountType to Initiator
    /// - Some(Platform) → Some(Initiator::Platform)
    /// - Some(Connected) → Some(Initiator::Connected)
    /// - Some(Standard) → None (standard flows don't need this context in responses)
    /// - None → None
    pub fn from_merchant_account_type(account_type: MerchantAccountType) -> Option<Self> {
        match account_type {
            MerchantAccountType::Platform => Some(Self::Platform),
            MerchantAccountType::Connected => Some(Self::Connected),
            MerchantAccountType::Standard => None,
        }
    }
}

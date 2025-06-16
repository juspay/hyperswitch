use crate::id_type;

/// Enum for different levels of authentication
#[derive(
    Clone,
    Debug,
    Hash,
    Eq,
    PartialEq,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    strum::EnumString,
)]
pub enum AuthInfo {
    /// OrgLevel: Authentication at the organization level
    OrgLevel {
        /// org_id: OrganizationId
        org_id: id_type::OrganizationId,
    },
    /// MerchantLevel: Authentication at the merchant level
    MerchantLevel {
        /// org_id: OrganizationId
        org_id: id_type::OrganizationId,
        /// merchant_ids: Vec<MerchantId>
        merchant_ids: Vec<id_type::MerchantId>,
    },
    /// ProfileLevel: Authentication at the profile level
    ProfileLevel {
        /// org_id: OrganizationId
        org_id: id_type::OrganizationId,
        /// merchant_id: MerchantId
        merchant_id: id_type::MerchantId,
        /// profile_ids: Vec<ProfileId>
        profile_ids: Vec<id_type::ProfileId>,
    },
}

/// Enum for different resource types supported in client secret
#[cfg(feature = "v2")]
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResourceId {
    /// Global Payment ID (Not exposed in api_models version of enum)
    Payment(id_type::GlobalPaymentId),
    /// Global Customer ID
    Customer(id_type::GlobalCustomerId),
    /// Global Payment Methods Session ID
    PaymentMethodSession(id_type::GlobalPaymentMethodSessionId),
}

#[cfg(feature = "v2")]
impl ResourceId {
    /// Get string representation of enclosed ID type
    pub fn to_str(&self) -> &str {
        match self {
            Self::Payment(id) => id.get_string_repr(),
            Self::Customer(id) => id.get_string_repr(),
            Self::PaymentMethodSession(id) => id.get_string_repr(),
        }
    }
}

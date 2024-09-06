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

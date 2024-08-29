use common_utils::id_type;

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
pub enum UserLevel {
    OrgLevel {
        org_id: id_type::OrganizationId,
    },
    MerchantLevel {
        org_id: id_type::OrganizationId,
        merchant_ids: Vec<id_type::MerchantId>,
    },
    ProfileLevel {
        org_id: id_type::OrganizationId,
        merchant_id: id_type::MerchantId,
        profile_ids: Vec<id_type::ProfileId>,
    },
}

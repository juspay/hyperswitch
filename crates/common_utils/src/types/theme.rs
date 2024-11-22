use crate::id_type;

/// Enum for having all the required lineage for every level.
/// Currently being used for theme related APIs and queries.
#[derive(Debug)]
pub enum ThemeLineage {
    // TODO: Add back Tenant variant when we introduce Tenant Variant in EntityType
    // /// Tenant lineage variant
    // Tenant {
    //     /// tenant_id: String
    //     tenant_id: String,
    // },
    /// Org lineage variant
    Organization {
        /// tenant_id: String
        tenant_id: String,
        /// org_id: OrganizationId
        org_id: id_type::OrganizationId,
    },
    /// Merchant lineage variant
    Merchant {
        /// tenant_id: String
        tenant_id: String,
        /// org_id: OrganizationId
        org_id: id_type::OrganizationId,
        /// merchant_id: MerchantId
        merchant_id: id_type::MerchantId,
    },
    /// Profile lineage variant
    Profile {
        /// tenant_id: String
        tenant_id: String,
        /// org_id: OrganizationId
        org_id: id_type::OrganizationId,
        /// merchant_id: MerchantId
        merchant_id: id_type::MerchantId,
        /// profile_id: ProfileId
        profile_id: id_type::ProfileId,
    },
}

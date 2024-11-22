use crate::id_type;

/// Enum for having all the required lineage for every level.
/// Currently being used for theme related APIs and queries.
#[derive(Debug)]
pub enum ThemeLineage {
    /// Tenant lineage variant
    Tenant {
        /// tenant_id: TenantId
        tenant_id: id_type::TenantId,
    },
    /// Org lineage variant
    Organization {
        /// tenant_id: TenantId
        tenant_id: id_type::TenantId,
        /// org_id: OrganizationId
        org_id: id_type::OrganizationId,
    },
    /// Merchant lineage variant
    Merchant {
        /// tenant_id: TenantId
        tenant_id: id_type::TenantId,
        /// org_id: OrganizationId
        org_id: id_type::OrganizationId,
        /// merchant_id: MerchantId
        merchant_id: id_type::MerchantId,
    },
    /// Profile lineage variant
    Profile {
        /// tenant_id: TenantId
        tenant_id: id_type::TenantId,
        /// org_id: OrganizationId
        org_id: id_type::OrganizationId,
        /// merchant_id: MerchantId
        merchant_id: id_type::MerchantId,
        /// profile_id: ProfileId
        profile_id: id_type::ProfileId,
    },
}

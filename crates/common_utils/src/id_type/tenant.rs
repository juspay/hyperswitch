use crate::{
    consts::{DEFAULT_GLOBAL_TENANT_ID, DEFAULT_TENANT},
    errors::{CustomResult, ValidationError},
};

crate::id_type!(
    TenantId,
    "A type for tenant_id that can be used for unique identifier for a tenant"
);
crate::impl_id_type_methods!(TenantId, "tenant_id");

// This is to display the `TenantId` as TenantId(abcd)
crate::impl_debug_id_type!(TenantId);
crate::impl_try_from_cow_str_id_type!(TenantId, "tenant_id");

crate::impl_serializable_secret_id_type!(TenantId);
crate::impl_queryable_id_type!(TenantId);
crate::impl_to_sql_from_sql_id_type!(TenantId);

impl TenantId {
    /// Get the default global tenant ID
    pub fn get_default_global_tenant_id() -> Self {
        Self(super::LengthId::new_unchecked(
            super::AlphaNumericId::new_unchecked(DEFAULT_GLOBAL_TENANT_ID.to_string()),
        ))
    }

    /// Get the default tenant ID
    pub fn get_default_tenant_id() -> Self {
        Self(super::LengthId::new_unchecked(
            super::AlphaNumericId::new_unchecked(DEFAULT_TENANT.to_string()),
        ))
    }

    /// Get tenant id from String
    pub fn try_from_string(tenant_id: String) -> CustomResult<Self, ValidationError> {
        Self::try_from(std::borrow::Cow::from(tenant_id))
    }
}

use super::{AlphaNumericId, LengthId};
use crate::errors::{CustomResult, ValidationError};

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
    /// Get tenant id from String
    pub fn wrap(tenant_id: String) -> CustomResult<Self, ValidationError> {
        Self::try_from(std::borrow::Cow::from(tenant_id))
    }

    /// Create a tenant ID without check
    pub fn new_unchecked(tenant_id: &str) -> Self {
        let alphanumeric_id = AlphaNumericId::new_unchecked(tenant_id.to_string());
        let length_id = LengthId::new_unchecked(alphanumeric_id);
        Self(length_id)
    }
}

use crate::errors::{CustomResult, ValidationError};

crate::id_type!(
    OrganizationId,
    "A type for organization_id that can be used for organization ids"
);
crate::impl_id_type_methods!(OrganizationId, "organization_id");

// This is to display the `OrganizationId` as OrganizationId(abcd)
crate::impl_debug_id_type!(OrganizationId);
crate::impl_default_id_type!(OrganizationId, "org");
crate::impl_try_from_cow_str_id_type!(OrganizationId, "organization_id");

crate::impl_generate_id_id_type!(OrganizationId, "org");
crate::impl_serializable_secret_id_type!(OrganizationId);
crate::impl_queryable_id_type!(OrganizationId);
crate::impl_to_sql_from_sql_id_type!(OrganizationId);

impl OrganizationId {
    /// Get an organization id from String
    pub fn try_from_string(org_id: String) -> CustomResult<Self, ValidationError> {
        Self::try_from(std::borrow::Cow::from(org_id))
    }
    /// get_authentication_service_eligible_key
    pub fn get_authentication_service_eligible_key(&self) -> String {
        format!("authentication_service_eligible_{}", self.get_string_repr())
    }

    /// fetch Get threeds_routing_region key for organization
    pub fn get_threeds_routing_region_uas_key(&self) -> String {
        format!("threeds_routing_region_uas_{}", self.get_string_repr())
    }

    /// Get should call PM modular service key for payment
    pub fn get_should_call_pm_modular_service_key(&self) -> String {
        format!("should_call_pm_modular_service_{}", self.get_string_repr())
    }
}

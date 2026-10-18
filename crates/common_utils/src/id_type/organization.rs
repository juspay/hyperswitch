use error_stack::ResultExt;

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

    /// Get should call PM modular service key for payment
    pub fn get_should_call_pm_modular_service_key(&self) -> String {
        format!("should_call_pm_modular_service_{}", self.get_string_repr())
    }

    /// Re-wraps this organization id as a [`super::MerchantId`] for use as an encryption identifier.
    pub fn as_merchant_key_identifier(&self) -> CustomResult<super::MerchantId, ValidationError> {
        super::MerchantId::try_from(std::borrow::Cow::Owned(self.get_string_repr().to_owned()))
            .change_context(ValidationError::InvalidValue {
                message: "organization id could not be used as a merchant key identifier"
                    .to_string(),
            })
    }
}

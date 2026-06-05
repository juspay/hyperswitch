use std::str::FromStr;

use crate::errors::{CustomResult, ValidationError};

crate::id_type!(
    MerchantConnectorAccountId,
    "A type for merchant_connector_id that can be used for merchant_connector_account ids"
);
crate::impl_id_type_methods!(MerchantConnectorAccountId, "merchant_connector_id");

// This is to display the `MerchantConnectorAccountId` as MerchantConnectorAccountId(abcd)
crate::impl_debug_id_type!(MerchantConnectorAccountId);
crate::impl_generate_id_id_type!(MerchantConnectorAccountId, "mca");
crate::impl_try_from_cow_str_id_type!(MerchantConnectorAccountId, "merchant_connector_id");

crate::impl_serializable_secret_id_type!(MerchantConnectorAccountId);
crate::impl_queryable_id_type!(MerchantConnectorAccountId);
crate::impl_to_sql_from_sql_id_type!(MerchantConnectorAccountId);

impl MerchantConnectorAccountId {
    /// Get a merchant connector account id from String
    pub fn wrap(merchant_connector_account_id: String) -> CustomResult<Self, ValidationError> {
        Self::try_from(std::borrow::Cow::from(merchant_connector_account_id))
    }

    /// Reverse-lookup id to find an authentication by its connector authentication
    /// id, scoped to this MCA so the key stays unique across connector accounts.
    pub fn get_authentication_connector_lookup_id(
        &self,
        connector_authentication_id: &str,
    ) -> String {
        format!(
            "auth_connector_{}_{}",
            self.get_string_repr(),
            connector_authentication_id
        )
    }
}

impl FromStr for MerchantConnectorAccountId {
    type Err = std::fmt::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_from(std::borrow::Cow::Owned(s.to_string())).map_err(|_| std::fmt::Error)
    }
}

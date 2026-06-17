crate::id_type!(
    AuthenticationId,
    "A type for authentication_id that can be used for authentication IDs"
);
crate::impl_id_type_methods!(AuthenticationId, "authentication_id");

// This is to display the `AuthenticationId` as AuthenticationId(abcd)
crate::impl_debug_id_type!(AuthenticationId);
crate::impl_try_from_cow_str_id_type!(AuthenticationId, "authentication_id");

crate::impl_serializable_secret_id_type!(AuthenticationId);
crate::impl_queryable_id_type!(AuthenticationId);
crate::impl_to_sql_from_sql_id_type!(AuthenticationId);

impl AuthenticationId {
    /// Generate Authentication Id from prefix
    pub fn generate_authentication_id(prefix: &'static str) -> Self {
        Self(crate::generate_ref_id_with_default_length(prefix))
    }

    /// Get external authentication request poll id
    pub fn get_external_authentication_request_poll_id(&self) -> String {
        format!("external_authentication_{}", self.get_string_repr())
    }
}

impl crate::events::ApiEventMetric for AuthenticationId {
    fn get_api_event_type(&self) -> Option<crate::events::ApiEventsType> {
        Some(crate::events::ApiEventsType::Authentication {
            authentication_id: self.clone(),
        })
    }
}

impl crate::events::ApiEventMetric for (super::MerchantId, AuthenticationId) {
    fn get_api_event_type(&self) -> Option<crate::events::ApiEventsType> {
        Some(crate::events::ApiEventsType::Authentication {
            authentication_id: self.1.clone(),
        })
    }
}

impl crate::events::ApiEventMetric for (&super::MerchantId, &AuthenticationId) {
    fn get_api_event_type(&self) -> Option<crate::events::ApiEventsType> {
        Some(crate::events::ApiEventsType::Authentication {
            authentication_id: self.1.clone(),
        })
    }
}

crate::impl_default_id_type!(AuthenticationId, "authentication");

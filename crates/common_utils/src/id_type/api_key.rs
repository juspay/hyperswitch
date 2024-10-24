crate::id_type!(
    ApiKeyId,
    "A type for key_id that can be used for API key IDs"
);
crate::impl_id_type_methods!(ApiKeyId, "key_id");

// This is to display the `ApiKeyId` as ApiKeyId(abcd)
crate::impl_debug_id_type!(ApiKeyId);
crate::impl_try_from_cow_str_id_type!(ApiKeyId, "key_id");

crate::impl_serializable_secret_id_type!(ApiKeyId);
crate::impl_queryable_id_type!(ApiKeyId);
crate::impl_to_sql_from_sql_id_type!(ApiKeyId);

impl ApiKeyId {
    /// Generate Api Key Id from prefix
    pub fn generate_key_id(prefix: &'static str) -> Self {
        Self(crate::generate_ref_id_with_default_length(prefix))
    }
}

impl crate::events::ApiEventMetric for ApiKeyId {
    fn get_api_event_type(&self) -> Option<crate::events::ApiEventsType> {
        Some(crate::events::ApiEventsType::ApiKey {
            key_id: self.clone(),
        })
    }
}

impl crate::events::ApiEventMetric for (super::MerchantId, ApiKeyId) {
    fn get_api_event_type(&self) -> Option<crate::events::ApiEventsType> {
        Some(crate::events::ApiEventsType::ApiKey {
            key_id: self.1.clone(),
        })
    }
}

impl crate::events::ApiEventMetric for (&super::MerchantId, &ApiKeyId) {
    fn get_api_event_type(&self) -> Option<crate::events::ApiEventsType> {
        Some(crate::events::ApiEventsType::ApiKey {
            key_id: self.1.clone(),
        })
    }
}

crate::impl_default_id_type!(ApiKeyId, "key");

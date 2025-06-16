crate::id_type!(
    ClientSecretId,
    "A type for key_id that can be used for Ephemeral key IDs"
);
crate::impl_id_type_methods!(ClientSecretId, "key_id");

// This is to display the `ClientSecretId` as ClientSecretId(abcd)
crate::impl_debug_id_type!(ClientSecretId);
crate::impl_try_from_cow_str_id_type!(ClientSecretId, "key_id");

crate::impl_generate_id_id_type!(ClientSecretId, "csi");
crate::impl_serializable_secret_id_type!(ClientSecretId);
crate::impl_queryable_id_type!(ClientSecretId);
crate::impl_to_sql_from_sql_id_type!(ClientSecretId);

#[cfg(feature = "v2")]
impl crate::events::ApiEventMetric for ClientSecretId {
    fn get_api_event_type(&self) -> Option<crate::events::ApiEventsType> {
        Some(crate::events::ApiEventsType::ClientSecret {
            key_id: self.clone(),
        })
    }
}

crate::impl_default_id_type!(ClientSecretId, "key");

impl ClientSecretId {
    /// Generate a key for redis
    pub fn generate_redis_key(&self) -> String {
        format!("cs_{}", self.get_string_repr())
    }
}

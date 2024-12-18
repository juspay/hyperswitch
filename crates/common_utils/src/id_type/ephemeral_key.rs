crate::id_type!(
    EphemeralKeyId,
    "A type for key_id that can be used for Ephemeral key IDs"
);
crate::impl_id_type_methods!(EphemeralKeyId, "key_id");

// This is to display the `EphemeralKeyId` as EphemeralKeyId(abcd)
crate::impl_debug_id_type!(EphemeralKeyId);
crate::impl_try_from_cow_str_id_type!(EphemeralKeyId, "key_id");

crate::impl_serializable_secret_id_type!(EphemeralKeyId);
crate::impl_queryable_id_type!(EphemeralKeyId);
crate::impl_to_sql_from_sql_id_type!(EphemeralKeyId);

impl EphemeralKeyId {
    /// Generate Ephemeral Key Id from prefix
    pub fn generate_key_id(prefix: &'static str) -> Self {
        Self(crate::generate_ref_id_with_default_length(prefix))
    }
}

impl crate::events::ApiEventMetric for EphemeralKeyId {
    fn get_api_event_type(&self) -> Option<crate::events::ApiEventsType> {
        Some(crate::events::ApiEventsType::EphemeralKey {
            key_id: self.clone(),
        })
    }
}

crate::impl_default_id_type!(EphemeralKeyId, "key");

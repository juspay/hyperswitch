crate::id_type!(
    ProfileId,
    "A type for profile_id that can be used for business profile ids"
);
crate::impl_id_type_methods!(ProfileId, "profile_id");

// This is to display the `ProfileId` as ProfileId(abcd)
crate::impl_debug_id_type!(ProfileId);
crate::impl_try_from_cow_str_id_type!(ProfileId, "profile_id");

crate::impl_generate_id_id_type!(ProfileId, "pro");
crate::impl_serializable_secret_id_type!(ProfileId);
crate::impl_queryable_id_type!(ProfileId);
crate::impl_to_sql_from_sql_id_type!(ProfileId);

impl crate::events::ApiEventMetric for ProfileId {
    fn get_api_event_type(&self) -> Option<crate::events::ApiEventsType> {
        Some(crate::events::ApiEventsType::BusinessProfile {
            profile_id: self.clone(),
        })
    }
}

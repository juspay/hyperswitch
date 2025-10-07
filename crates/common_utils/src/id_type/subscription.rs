crate::id_type!(
    SubscriptionId,
    " A type for subscription_id that can be used for subscription ids"
);

crate::impl_id_type_methods!(SubscriptionId, "subscription_id");

// This is to display the `SubscriptionId` as SubscriptionId(subs)
crate::impl_debug_id_type!(SubscriptionId);
crate::impl_try_from_cow_str_id_type!(SubscriptionId, "subscription_id");

crate::impl_generate_id_id_type!(SubscriptionId, "sub");
crate::impl_serializable_secret_id_type!(SubscriptionId);
crate::impl_queryable_id_type!(SubscriptionId);
crate::impl_to_sql_from_sql_id_type!(SubscriptionId);

impl crate::events::ApiEventMetric for SubscriptionId {
    fn get_api_event_type(&self) -> Option<crate::events::ApiEventsType> {
        Some(crate::events::ApiEventsType::Subscription)
    }
}

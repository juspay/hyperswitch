crate::id_type!(
    RoutingId,
    " A type for routing_id that can be used for routing ids"
);

crate::impl_id_type_methods!(RoutingId, "routing_id");

// This is to display the `RoutingId` as RoutingId(abcd)
crate::impl_debug_id_type!(RoutingId);
crate::impl_try_from_cow_str_id_type!(RoutingId, "routing_id");

crate::impl_generate_id_id_type!(RoutingId, "routing");
crate::impl_serializable_secret_id_type!(RoutingId);
crate::impl_queryable_id_type!(RoutingId);
crate::impl_to_sql_from_sql_id_type!(RoutingId);

impl crate::events::ApiEventMetric for RoutingId {
    fn get_api_event_type(&self) -> Option<crate::events::ApiEventsType> {
        Some(crate::events::ApiEventsType::Routing)
    }
}

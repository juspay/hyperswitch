crate::id_type!(
    ResourceId,
    "A type for resource_id that can be used for generic linkable resource ids"
);
crate::impl_id_type_methods!(ResourceId, "resource_id");

// This is to display the `ResourceId` as ResourceId(abcd)
crate::impl_debug_id_type!(ResourceId);
crate::impl_default_id_type!(ResourceId, "res");
crate::impl_try_from_cow_str_id_type!(ResourceId, "resource_id");

crate::impl_generate_id_id_type!(ResourceId, "res");
crate::impl_serializable_secret_id_type!(ResourceId);
crate::impl_queryable_id_type!(ResourceId);
crate::impl_to_sql_from_sql_id_type!(ResourceId);

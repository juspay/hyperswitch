crate::id_type!(
    CustomerId,
    "A type for customer_id that can be used for customer ids"
);
crate::impl_id_type_methods!(CustomerId, "customer_id");

// This is to display the `CustomerId` as CustomerId(abcd)
crate::impl_debug_id_type!(CustomerId);
crate::impl_default_id_type!(CustomerId, "cus");
crate::impl_try_from_cow_str_id_type!(CustomerId, "customer_id");

crate::impl_generate_id_id_type!(CustomerId, "cus");
crate::impl_serializable_secret_id_type!(CustomerId);
#[cfg(feature = "diesel")]
crate::impl_queryable_id_type!(CustomerId);
#[cfg(feature = "diesel")]
crate::impl_to_sql_from_sql_id_type!(CustomerId);

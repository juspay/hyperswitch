crate::id_type!(
    PayoutId,
    "A domain type for payout_id that can be used for payout ids"
);
crate::impl_id_type_methods!(PayoutId, "payout_id");
crate::impl_debug_id_type!(PayoutId);
crate::impl_try_from_cow_str_id_type!(PayoutId, "payout_id");
crate::impl_generate_id_id_type!(PayoutId, "payout");
crate::impl_queryable_id_type!(PayoutId);
crate::impl_to_sql_from_sql_id_type!(PayoutId);

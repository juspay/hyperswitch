crate::id_type!(RefundReferenceId, "A type for refund_reference_id");
crate::impl_id_type_methods!(RefundReferenceId, "refund_reference_id");

// This is to display the `RefundReferenceId` as RefundReferenceId(abcd)
crate::impl_debug_id_type!(RefundReferenceId);
crate::impl_try_from_cow_str_id_type!(RefundReferenceId, "refund_reference_id");

// Database related implementations so that this field can be used directly in the database tables
crate::impl_queryable_id_type!(RefundReferenceId);
crate::impl_to_sql_from_sql_id_type!(RefundReferenceId);

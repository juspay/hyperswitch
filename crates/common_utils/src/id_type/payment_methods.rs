crate::id_type!(
    PaymentMethodId,
    "A type for payment_method_id that can be used for payment method ids"
);
crate::impl_id_type_methods!(PaymentMethodId, "payment_method_id");

// This is to display the `PaymentMethodId` as PaymentMethodId(abcd)
crate::impl_debug_id_type!(PaymentMethodId);
crate::impl_default_id_type!(PaymentMethodId, "pm");
crate::impl_try_from_cow_str_id_type!(PaymentMethodId, "payment_method_id");

crate::impl_serializable_secret_id_type!(PaymentMethodId);
crate::impl_queryable_id_type!(PaymentMethodId);
crate::impl_to_sql_from_sql_id_type!(PaymentMethodId);

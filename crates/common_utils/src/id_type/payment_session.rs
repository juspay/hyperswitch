crate::id_type!(
    PaymentSessionId,
    "A type for payment_session_id that can be used for payment sessions"
);
crate::impl_id_type_methods!(PaymentSessionId, "payment_session_id");

// This is to display the `PaymentSessionId` as PaymentSessionId(abcd)
crate::impl_debug_id_type!(PaymentSessionId);
crate::impl_generate_id_id_type!(PaymentSessionId, "pay_sess");
crate::impl_try_from_cow_str_id_type!(PaymentSessionId, "payment_session_id");

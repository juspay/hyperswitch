crate::id_type!(
    ClientSessionId,
    "A type for client_session_id that can be used for payment sessions"
);
crate::impl_id_type_methods!(ClientSessionId, "client_session_id");

// This is to display the `ClientSessionId` as ClientSessionId(abcd)
crate::impl_debug_id_type!(ClientSessionId);
crate::impl_generate_id_id_type!(ClientSessionId, "client_sess");
crate::impl_try_from_cow_str_id_type!(ClientSessionId, "client_session_id");

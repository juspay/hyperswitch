// In crates/common_utils/src/id_type/payout.rs

// The macros like `crate::id_type` are defined within the `common_utils` crate,
// likely in `common_utils/src/id_type/mod.rs` or a file included there,
// making them accessible via `crate::` from within `payout.rs`.
// Specific type imports like `AlphaNumericId` or `LengthId` are encapsulated
// by these macros or are part of the `id_type` prelude if one exists.

crate::id_type!(
    PayoutId,
    "A type for payout_id that can be used for payout ids"
);
crate::impl_id_type_methods!(PayoutId, "payout_id");

// This is to display the `PayoutId` as PayoutId(abcd)
crate::impl_debug_id_type!(PayoutId);
// Using the user-specified prefix "payout_"
crate::impl_default_id_type!(PayoutId, "payout_");
crate::impl_try_from_cow_str_id_type!(PayoutId, "payout_id");

// Database related implementations so that this field can be used directly in the database tables
crate::impl_queryable_id_type!(PayoutId);
crate::impl_to_sql_from_sql_id_type!(PayoutId);

impl std::fmt::Display for PayoutId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.get_string_repr())
    }
}

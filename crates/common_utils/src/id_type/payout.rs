use crate::errors::ValidationError;

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

crate::id_type!(PayoutResourceId, "A type for payout_resource_id");
crate::impl_id_type_methods!(PayoutResourceId, "payout_resource_id");

// This is to display the `PayoutResourceId` as PayoutResourceId(abcd)
crate::impl_debug_id_type!(PayoutResourceId);
crate::impl_try_from_cow_str_id_type!(PayoutResourceId, "payout_resource_id");

// Database related implementations so that this field can be used directly in the database tables
crate::impl_queryable_id_type!(PayoutResourceId);
crate::impl_to_sql_from_sql_id_type!(PayoutResourceId);

crate::id_type!(PayoutReferenceId, "A type for payout_reference_id");
crate::impl_id_type_methods!(PayoutReferenceId, "payout_reference_id");

// This is to display the `PayoutReferenceId` as PayoutReferenceId(abcd)
crate::impl_debug_id_type!(PayoutReferenceId);
crate::impl_try_from_cow_str_id_type!(PayoutReferenceId, "payout_reference_id");

// Database related implementations so that this field can be used directly in the database tables
crate::impl_queryable_id_type!(PayoutReferenceId);
crate::impl_to_sql_from_sql_id_type!(PayoutReferenceId);

impl std::str::FromStr for PayoutReferenceId {
    type Err = error_stack::Report<ValidationError>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let cow_string = std::borrow::Cow::Owned(s.to_string());
        Self::try_from(cow_string)
    }
}

impl std::str::FromStr for PayoutResourceId {
    type Err = error_stack::Report<ValidationError>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let cow_string = std::borrow::Cow::Owned(s.to_string());
        Self::try_from(cow_string)
    }
}

use std::str::FromStr;

crate::id_type!(
    RelayId,
    "A type for relay_id that can be used for relay ids"
);
crate::impl_id_type_methods!(RelayId, "relay_id");

crate::impl_try_from_cow_str_id_type!(RelayId, "relay_id");
crate::impl_generate_id_id_type!(RelayId, "relay");
crate::impl_serializable_secret_id_type!(RelayId);
crate::impl_queryable_id_type!(RelayId);
crate::impl_to_sql_from_sql_id_type!(RelayId);

crate::impl_debug_id_type!(RelayId);

impl FromStr for RelayId {
    type Err = error_stack::Report<crate::errors::ValidationError>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let cow_string = std::borrow::Cow::Owned(s.to_string());
        Self::try_from(cow_string)
    }
}

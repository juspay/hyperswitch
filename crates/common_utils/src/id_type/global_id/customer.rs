use error_stack::ResultExt;

use crate::{errors, generate_id_with_default_len, generate_time_ordered_id_without_prefix, types};

crate::global_id_type!(
    GlobalCustomerId,
    "A global id that can be used to identify a customer.

The format will be `<cell_id>_<entity_prefix>_<time_ordered_id>`.

Example: `cell1_cus_uu1a2b3c4d5e6f7g8h9i0j1k2l3m4n5o6p`"
);

// Database related implementations so that this field can be used directly in the database tables
crate::impl_queryable_id_type!(GlobalCustomerId);
crate::impl_to_sql_from_sql_global_id_type!(GlobalCustomerId);

impl GlobalCustomerId {
    /// Get string representation of the id
    pub fn get_string_repr(&self) -> &str {
        self.0.get_string_repr()
    }

    /// Generate a new GlobalCustomerId from a cell id
    pub fn generate(cell_id: &crate::id_type::CellId) -> Self {
        let global_id = super::GlobalId::generate(cell_id, super::GlobalEntity::Customer);
        Self(global_id)
    }
}

impl TryFrom<GlobalCustomerId> for crate::id_type::CustomerId {
    type Error = error_stack::Report<crate::errors::ValidationError>;

    fn try_from(value: GlobalCustomerId) -> Result<Self, Self::Error> {
        Self::try_from(std::borrow::Cow::from(value.get_string_repr().to_owned()))
    }
}

impl crate::events::ApiEventMetric for GlobalCustomerId {
    fn get_api_event_type(&self) -> Option<crate::events::ApiEventsType> {
        Some(crate::events::ApiEventsType::Customer {
            customer_id: Some(self.clone()),
        })
    }
}

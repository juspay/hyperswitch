use std::borrow::Cow;

use error_stack::ResultExt;

use crate::errors::{CustomResult, ValidationError};

crate::global_id_type!(
    GlobalTokenId,
    "A global id that can be used to identify a token.

The format will be `<cell_id>_<entity_prefix>_<time_ordered_id>`.

Example: `cell1_tok_uu1a2b3c4d5e6f7g8h9i0j1k2l3m4n5o6p`"
);

// Database related implementations so that this field can be used directly in the database tables
crate::impl_queryable_id_type!(GlobalTokenId);
crate::impl_to_sql_from_sql_global_id_type!(GlobalTokenId);

impl GlobalTokenId {
    /// Get string representation of the id
    pub fn get_string_repr(&self) -> &str {
        self.0.get_string_repr()
    }

    ///Get GlobalTokenId from a string
    pub fn from_string(token_string: &str) -> CustomResult<Self, ValidationError> {
        let token = super::GlobalId::from_string(Cow::Owned(token_string.to_string()))
            .change_context(ValidationError::IncorrectValueProvided {
                field_name: "GlobalTokenId",
            })?;
        Ok(Self(token))
    }

    /// Generate a new GlobalTokenId from a cell id
    pub fn generate(cell_id: &crate::id_type::CellId) -> Self {
        let global_id = super::GlobalId::generate(cell_id, super::GlobalEntity::Token);
        Self(global_id)
    }
}

impl crate::events::ApiEventMetric for GlobalTokenId {
    fn get_api_event_type(&self) -> Option<crate::events::ApiEventsType> {
        Some(crate::events::ApiEventsType::Token {
            token_id: Some(self.clone()),
        })
    }
}

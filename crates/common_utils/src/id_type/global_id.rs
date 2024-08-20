#![allow(unused)]

use error_stack::ResultExt;

use crate::{
    consts::{
        CELL_IDENTIFIER_LENGTH, MAX_ALLOWED_MERCHANT_REFERENCE_ID_LENGTH,
        MIN_REQUIRED_MERCHANT_REFERENCE_ID_LENGTH,
    },
    errors, generate_time_ordered_id,
    id_type::LengthId,
};

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
/// A global id that can be used to identify any entity
/// This id will have information about the entity and cell in a distributed system architecture
struct GlobalId(
    LengthId<MAX_ALLOWED_MERCHANT_REFERENCE_ID_LENGTH, MIN_REQUIRED_MERCHANT_REFERENCE_ID_LENGTH>,
);

/// Entities that can be identified by a global id
enum GlobalEntity {
    Customer,
    Payment,
}

impl GlobalEntity {
    fn prefix(&self) -> &'static str {
        match self {
            Self::Customer => "cus",
            Self::Payment => "pay",
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct CellId(LengthId<CELL_IDENTIFIER_LENGTH, CELL_IDENTIFIER_LENGTH>);

impl CellId {
    /// Create a new cell id from a string
    pub fn from_str(cell_id_string: &str) -> error_stack::Result<Self, errors::ValidationError> {
        let trimmed_input_string = cell_id_string.trim().to_string();
        let alphanumeric_id = super::AlphaNumericId::from(trimmed_input_string.into())
            .change_context(errors::ValidationError::InvalidValue {
                message: "cell_id contains invalid characters".to_string(),
            })?;

        let length_id = LengthId::from_alphanumeric_id(alphanumeric_id).change_context(
            errors::ValidationError::InvalidValue {
                message: format!("The length of `cell_id` should be {CELL_IDENTIFIER_LENGTH}"),
            },
        )?;

        Ok(Self(length_id))
    }

    /// Get the string representation of the cell id
    fn get_string_repr(&self) -> &str {
        &self.0 .0 .0
    }
}

impl GlobalId {
    /// Create a new global id from entity and cell information
    /// The entity prefix is used to identify the entity, `cus` for customers, `pay`` for payments etc.
    pub fn generate(entity: GlobalEntity, cell_id: CellId) -> Self {
        let prefix = format!("{}_{}", entity.prefix(), cell_id.get_string_repr());
        let id = generate_time_ordered_id(&prefix);
        let alphanumeric_id = super::AlphaNumericId::new_unchecked(id);
        Self(LengthId::new_unchecked(alphanumeric_id))
    }
}

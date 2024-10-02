use error_stack::ResultExt;

use crate::{errors, generate_id_with_default_len};

crate::global_id_type!(
    GlobalPaymentId,
    "A global id that can be used to identify a payment
    The format will be `<cell_id>_<entity_prefix>_<time_ordered_id>`
    example - cell1_pay_uu1a2b3c4d5e6f7g8h9i0j1k2l3m4n5o6p"
);

// Database related implementations so that this field can be used directly in the database tables
crate::impl_queryable_id_type!(GlobalPaymentId);
crate::impl_to_sql_from_sql_global_id_type!(GlobalPaymentId);

#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum GlobalPaymentIdError {
    #[error("Failed to construct GlobalPaymentId")]
    ConstructionError,
}

impl GlobalPaymentId {
    /// Get string representation of the id
    pub fn get_string_repr(&self) -> &str {
        self.0.get_string_repr()
    }

    /// Generate a new GlobalPaymentId from a cell id
    pub fn generate(cell_id: &str) -> error_stack::Result<Self, GlobalPaymentIdError> {
        let cell_id = super::CellId::from_str(cell_id)
            .change_context(GlobalPaymentIdError::ConstructionError)
            .attach_printable("Error generating GlobalPaymentId")?;
        let global_id = super::GlobalId::generate(cell_id, super::GlobalEntity::Payment);
        Ok(Self(global_id))
    }
}

// TODO: refactor the macro to include this id use case as well
impl TryFrom<std::borrow::Cow<'static, str>> for GlobalPaymentId {
    type Error = error_stack::Report<errors::ValidationError>;
    fn try_from(value: std::borrow::Cow<'static, str>) -> Result<Self, Self::Error> {
        use error_stack::ResultExt;
        let merchant_ref_id = super::GlobalId::from_string(value).change_context(
            errors::ValidationError::IncorrectValueProvided {
                field_name: "payment_id",
            },
        )?;
        Ok(Self(merchant_ref_id))
    }
}

crate::global_id_type!(
    GlobalAttemptId,
    "A global id that can be used to identify a payment attempt"
);

// Database related implementations so that this field can be used directly in the database tables
crate::impl_queryable_id_type!(GlobalAttemptId);
crate::impl_to_sql_from_sql_global_id_type!(GlobalAttemptId);

impl GlobalAttemptId {
    /// Get string representation of the id
    pub fn get_string_repr(&self) -> &str {
        self.0.get_string_repr()
    }
}

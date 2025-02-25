use error_stack::ResultExt;

use crate::{errors, generate_id_with_default_len, generate_time_ordered_id_without_prefix, types};

/// A global id that can be used to identify a refund
#[derive(
    Debug,
    Clone,
    Hash,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    diesel::expression::AsExpression,
)]
#[diesel(sql_type = diesel::sql_types::Text)]
pub struct GlobalRefundId(super::GlobalId);

// Database related implementations so that this field can be used directly in the database tables
crate::impl_queryable_id_type!(GlobalRefundId);

impl GlobalRefundId {
    /// Get string representation of the id
    pub fn get_string_repr(&self) -> &str {
        self.0.get_string_repr()
    }

    /// Generate a new GlobalRefundId from a cell id
    pub fn generate(cell_id: &crate::id_type::CellId) -> Self {
        let global_id = super::GlobalId::generate(cell_id, super::GlobalEntity::Refund);
        Self(global_id)
    }
}

// TODO: refactor the macro to include this id use case as well
impl TryFrom<std::borrow::Cow<'static, str>> for GlobalRefundId {
    type Error = error_stack::Report<errors::ValidationError>;
    fn try_from(value: std::borrow::Cow<'static, str>) -> Result<Self, Self::Error> {
        use error_stack::ResultExt;
        let merchant_ref_id = super::GlobalId::from_string(value).change_context(
            errors::ValidationError::IncorrectValueProvided {
                field_name: "refund_id",
            },
        )?;
        Ok(Self(merchant_ref_id))
    }
}

// TODO: refactor the macro to include this id use case as well
impl<DB> diesel::serialize::ToSql<diesel::sql_types::Text, DB> for GlobalRefundId
where
    DB: diesel::backend::Backend,
    super::GlobalId: diesel::serialize::ToSql<diesel::sql_types::Text, DB>,
{
    fn to_sql<'b>(
        &'b self,
        out: &mut diesel::serialize::Output<'b, '_, DB>,
    ) -> diesel::serialize::Result {
        self.0.to_sql(out)
    }
}

impl<DB> diesel::deserialize::FromSql<diesel::sql_types::Text, DB> for GlobalRefundId
where
    DB: diesel::backend::Backend,
    super::GlobalId: diesel::deserialize::FromSql<diesel::sql_types::Text, DB>,
{
    fn from_sql(value: DB::RawValue<'_>) -> diesel::deserialize::Result<Self> {
        super::GlobalId::from_sql(value).map(Self)
    }
}

use diesel::{backend::Backend, deserialize::FromSql, serialize::ToSql, sql_types};
use error_stack::ResultExt;

use crate::{
    errors,
    errors::CustomResult,
    id_type::global_id::{CellId, GlobalEntity, GlobalId, GlobalIdError},
};

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
pub struct GlobalPaymentMethodId(GlobalId);

#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum GlobalPaymentMethodIdError {
    #[error("Failed to construct GlobalPaymentMethodId")]
    ConstructionError,
}

impl GlobalPaymentMethodId {
    /// Create a new GlobalPaymentMethodId from celll id information
    pub fn generate(cell_id: &str) -> error_stack::Result<Self, GlobalPaymentMethodIdError> {
        let cell_id = CellId::from_str(cell_id)
            .change_context(GlobalPaymentMethodIdError::ConstructionError)
            .attach_printable("Failed to construct CellId from str")?;
        let global_id = GlobalId::generate(cell_id, GlobalEntity::PaymentMethod);
        Ok(Self(global_id))
    }

    pub fn get_string_repr(&self) -> &str {
        self.0.get_string_repr()
    }

    pub fn generate_from_string(value: String) -> CustomResult<Self, GlobalPaymentMethodIdError> {
        let id = GlobalId::from_string(value.into())
            .change_context(GlobalPaymentMethodIdError::ConstructionError)?;
        Ok(Self(id))
    }
}

impl<DB> diesel::Queryable<sql_types::Text, DB> for GlobalPaymentMethodId
where
    DB: diesel::backend::Backend,
    Self: diesel::deserialize::FromSql<diesel::sql_types::Text, DB>,
{
    type Row = Self;
    fn build(row: Self::Row) -> diesel::deserialize::Result<Self> {
        Ok(row)
    }
}

impl<DB> ToSql<sql_types::Text, DB> for GlobalPaymentMethodId
where
    DB: Backend,
    GlobalId: ToSql<sql_types::Text, DB>,
{
    fn to_sql<'b>(
        &'b self,
        out: &mut diesel::serialize::Output<'b, '_, DB>,
    ) -> diesel::serialize::Result {
        self.0.to_sql(out)
    }
}

impl<DB> FromSql<sql_types::Text, DB> for GlobalPaymentMethodId
where
    DB: Backend,
    GlobalId: FromSql<sql_types::Text, DB>,
{
    fn from_sql(value: DB::RawValue<'_>) -> diesel::deserialize::Result<Self> {
        let global_id = GlobalId::from_sql(value)?;
        Ok(Self(global_id))
    }
}

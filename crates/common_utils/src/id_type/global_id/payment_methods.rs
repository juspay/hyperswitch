use error_stack::ResultExt;

use crate::{
    errors::CustomResult,
    id_type::global_id::{CellId, GlobalEntity, GlobalId},
};

/// A global id that can be used to identify a payment method
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

/// A global id that can be used to identify a payment method session
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
pub struct GlobalPaymentMethodSessionId(GlobalId);

#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum GlobalPaymentMethodIdError {
    #[error("Failed to construct GlobalPaymentMethodId")]
    ConstructionError,
}

#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum GlobalPaymentMethodSessionIdError {
    #[error("Failed to construct GlobalPaymentMethodSessionId")]
    ConstructionError,
}

impl GlobalPaymentMethodSessionId {
    /// Create a new GlobalPaymentMethodSessionId from cell id information
    pub fn generate(
        cell_id: &CellId,
    ) -> error_stack::Result<Self, GlobalPaymentMethodSessionIdError> {
        let global_id = GlobalId::generate(cell_id, GlobalEntity::PaymentMethodSession);
        Ok(Self(global_id))
    }

    /// Get the string representation of the id
    pub fn get_string_repr(&self) -> &str {
        self.0.get_string_repr()
    }

    /// Construct a redis key from the id to be stored in redis
    pub fn get_redis_key(&self) -> String {
        format!("payment_method_session:{}", self.get_string_repr())
    }
}

#[cfg(feature = "v2")]
impl crate::events::ApiEventMetric for GlobalPaymentMethodSessionId {
    fn get_api_event_type(&self) -> Option<crate::events::ApiEventsType> {
        Some(crate::events::ApiEventsType::PaymentMethodSession {
            payment_method_session_id: self.clone(),
        })
    }
}

impl crate::events::ApiEventMetric for GlobalPaymentMethodId {
    fn get_api_event_type(&self) -> Option<crate::events::ApiEventsType> {
        Some(
            crate::events::ApiEventsType::PaymentMethodListForPaymentMethods {
                payment_method_id: self.clone(),
            },
        )
    }
}

impl GlobalPaymentMethodId {
    /// Create a new GlobalPaymentMethodId from cell id information
    pub fn generate(cell_id: &CellId) -> error_stack::Result<Self, GlobalPaymentMethodIdError> {
        let global_id = GlobalId::generate(cell_id, GlobalEntity::PaymentMethod);
        Ok(Self(global_id))
    }

    /// Get string representation of the id
    pub fn get_string_repr(&self) -> &str {
        self.0.get_string_repr()
    }

    /// Construct a new GlobalPaymentMethodId from a string
    pub fn generate_from_string(value: String) -> CustomResult<Self, GlobalPaymentMethodIdError> {
        let id = GlobalId::from_string(value.into())
            .change_context(GlobalPaymentMethodIdError::ConstructionError)?;
        Ok(Self(id))
    }
}

impl<DB> diesel::Queryable<diesel::sql_types::Text, DB> for GlobalPaymentMethodId
where
    DB: diesel::backend::Backend,
    Self: diesel::deserialize::FromSql<diesel::sql_types::Text, DB>,
{
    type Row = Self;
    fn build(row: Self::Row) -> diesel::deserialize::Result<Self> {
        Ok(row)
    }
}

impl<DB> diesel::serialize::ToSql<diesel::sql_types::Text, DB> for GlobalPaymentMethodId
where
    DB: diesel::backend::Backend,
    GlobalId: diesel::serialize::ToSql<diesel::sql_types::Text, DB>,
{
    fn to_sql<'b>(
        &'b self,
        out: &mut diesel::serialize::Output<'b, '_, DB>,
    ) -> diesel::serialize::Result {
        self.0.to_sql(out)
    }
}

impl<DB> diesel::deserialize::FromSql<diesel::sql_types::Text, DB> for GlobalPaymentMethodId
where
    DB: diesel::backend::Backend,
    GlobalId: diesel::deserialize::FromSql<diesel::sql_types::Text, DB>,
{
    fn from_sql(value: DB::RawValue<'_>) -> diesel::deserialize::Result<Self> {
        let global_id = GlobalId::from_sql(value)?;
        Ok(Self(global_id))
    }
}

impl<DB> diesel::Queryable<diesel::sql_types::Text, DB> for GlobalPaymentMethodSessionId
where
    DB: diesel::backend::Backend,
    Self: diesel::deserialize::FromSql<diesel::sql_types::Text, DB>,
{
    type Row = Self;
    fn build(row: Self::Row) -> diesel::deserialize::Result<Self> {
        Ok(row)
    }
}

impl<DB> diesel::serialize::ToSql<diesel::sql_types::Text, DB> for GlobalPaymentMethodSessionId
where
    DB: diesel::backend::Backend,
    GlobalId: diesel::serialize::ToSql<diesel::sql_types::Text, DB>,
{
    fn to_sql<'b>(
        &'b self,
        out: &mut diesel::serialize::Output<'b, '_, DB>,
    ) -> diesel::serialize::Result {
        self.0.to_sql(out)
    }
}

impl<DB> diesel::deserialize::FromSql<diesel::sql_types::Text, DB> for GlobalPaymentMethodSessionId
where
    DB: diesel::backend::Backend,
    GlobalId: diesel::deserialize::FromSql<diesel::sql_types::Text, DB>,
{
    fn from_sql(value: DB::RawValue<'_>) -> diesel::deserialize::Result<Self> {
        let global_id = GlobalId::from_sql(value)?;
        Ok(Self(global_id))
    }
}

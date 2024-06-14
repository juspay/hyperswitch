use std::{borrow::Cow, fmt::Debug};

use diesel::{
    backend::Backend,
    deserialize::FromSql,
    expression::AsExpression,
    serialize::{Output, ToSql},
    sql_types, Queryable,
};
use error_stack::{Result, ResultExt};
use serde::{Deserialize, Serialize};

use crate::{
    consts::{MAX_ALLOWED_MERCHANT_REFERENCE_ID_LENGTH, MIN_REQUIRED_MERCHANT_REFERENCE_ID_LENGTH},
    errors, generate_customer_id_of_default_length,
    id_type::MerchantReferenceId,
};

/// A type for customer_id that can be used for customer ids
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, AsExpression)]
#[diesel(sql_type = sql_types::Text)]
pub struct CustomerId(
    MerchantReferenceId<
        MAX_ALLOWED_MERCHANT_REFERENCE_ID_LENGTH,
        MIN_REQUIRED_MERCHANT_REFERENCE_ID_LENGTH,
    >,
);

impl Default for CustomerId {
    fn default() -> Self {
        generate_customer_id_of_default_length()
    }
}

/// This is to display the `CustomerId` as CustomerId(abcd)
impl Debug for CustomerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("CustomerId").field(&self.0 .0 .0).finish()
    }
}

impl<DB> Queryable<sql_types::Text, DB> for CustomerId
where
    DB: Backend,
    Self: FromSql<sql_types::Text, DB>,
{
    type Row = Self;

    fn build(row: Self::Row) -> diesel::deserialize::Result<Self> {
        Ok(row)
    }
}

impl CustomerId {
    pub(crate) fn new(
        merchant_ref_id: MerchantReferenceId<
            MAX_ALLOWED_MERCHANT_REFERENCE_ID_LENGTH,
            MIN_REQUIRED_MERCHANT_REFERENCE_ID_LENGTH,
        >,
    ) -> Self {
        Self(merchant_ref_id)
    }

    /// Get the string representation of customer id
    pub fn get_string_repr(&self) -> &str {
        &self.0 .0 .0
    }

    /// Create a Customer id from string
    pub fn from(input_string: Cow<'static, str>) -> Result<Self, errors::ValidationError> {
        let merchant_ref_id = MerchantReferenceId::from(input_string).change_context(
            errors::ValidationError::IncorrectValueProvided {
                field_name: "customer_id",
            },
        )?;

        Ok(Self(merchant_ref_id))
    }
}

impl masking::SerializableSecret for CustomerId {}

impl<DB> ToSql<sql_types::Text, DB> for CustomerId
where
    DB: Backend,
    MerchantReferenceId<
        MAX_ALLOWED_MERCHANT_REFERENCE_ID_LENGTH,
        MIN_REQUIRED_MERCHANT_REFERENCE_ID_LENGTH,
    >: ToSql<sql_types::Text, DB>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> diesel::serialize::Result {
        self.0.to_sql(out)
    }
}

impl<DB> FromSql<sql_types::Text, DB> for CustomerId
where
    DB: Backend,
    MerchantReferenceId<
        MAX_ALLOWED_MERCHANT_REFERENCE_ID_LENGTH,
        MIN_REQUIRED_MERCHANT_REFERENCE_ID_LENGTH,
    >: FromSql<sql_types::Text, DB>,
{
    fn from_sql(value: DB::RawValue<'_>) -> diesel::deserialize::Result<Self> {
        MerchantReferenceId::<
            MAX_ALLOWED_MERCHANT_REFERENCE_ID_LENGTH,
            MIN_REQUIRED_MERCHANT_REFERENCE_ID_LENGTH,
        >::from_sql(value)
        .map(Self)
    }
}

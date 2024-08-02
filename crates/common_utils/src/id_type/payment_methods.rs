use std::{
    borrow::Cow,
    fmt::{Debug, Display},
};

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
    errors, generate_payment_method_id_of_default_length,
    id_type::LengthId,
};

/// A length id type for payment_method_id that can be used for payment method structs
#[derive(Clone, Serialize, Deserialize, Hash, PartialEq, Eq, AsExpression)]
#[diesel(sql_type = sql_types::Text)]
pub struct PaymentMethodId(
    LengthId<MAX_ALLOWED_MERCHANT_REFERENCE_ID_LENGTH, MIN_REQUIRED_MERCHANT_REFERENCE_ID_LENGTH>,
);

impl Default for PaymentMethodId {
    fn default() -> Self {
        generate_payment_method_id_of_default_length()
    }
}

/// This should be temporary, we should not have direct impl of Display for payment method id
impl Display for PaymentMethodId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.get_string_repr())
    }
}

/// This is to display the `PaymentMethodId` as PaymentMethodId(abcd)
impl Debug for PaymentMethodId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("PaymentMethodId")
            .field(&self.0 .0 .0)
            .finish()
    }
}

impl<DB> Queryable<sql_types::Text, DB> for PaymentMethodId
where
    DB: Backend,
    Self: FromSql<sql_types::Text, DB>,
{
    type Row = Self;

    fn build(row: Self::Row) -> diesel::deserialize::Result<Self> {
        Ok(row)
    }
}

impl PaymentMethodId {
    pub(crate) fn new(
        merchant_ref_id: LengthId<
            MAX_ALLOWED_MERCHANT_REFERENCE_ID_LENGTH,
            MIN_REQUIRED_MERCHANT_REFERENCE_ID_LENGTH,
        >,
    ) -> Self {
        Self(merchant_ref_id)
    }

    /// Get the string representation of payment_method id
    pub fn get_string_repr(&self) -> &str {
        &self.0 .0 .0
    }

    /// Create a payment_method id from string
    pub fn from(input_string: Cow<'static, str>) -> Result<Self, errors::ValidationError> {
        let merchant_ref_id = LengthId::from(input_string).change_context(
            errors::ValidationError::IncorrectValueProvided {
                field_name: "payment_method_id",
            },
        )?;

        Ok(Self(merchant_ref_id))
    }
}

impl masking::SerializableSecret for PaymentMethodId {}

impl<DB> ToSql<sql_types::Text, DB> for PaymentMethodId
where
    DB: Backend,
    LengthId<MAX_ALLOWED_MERCHANT_REFERENCE_ID_LENGTH, MIN_REQUIRED_MERCHANT_REFERENCE_ID_LENGTH>:
        ToSql<sql_types::Text, DB>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> diesel::serialize::Result {
        self.0.to_sql(out)
    }
}

impl<DB> FromSql<sql_types::Text, DB> for PaymentMethodId
where
    DB: Backend,
    LengthId<MAX_ALLOWED_MERCHANT_REFERENCE_ID_LENGTH, MIN_REQUIRED_MERCHANT_REFERENCE_ID_LENGTH>:
        FromSql<sql_types::Text, DB>,
{
    fn from_sql(value: DB::RawValue<'_>) -> diesel::deserialize::Result<Self> {
        LengthId::<
            MAX_ALLOWED_MERCHANT_REFERENCE_ID_LENGTH,
            MIN_REQUIRED_MERCHANT_REFERENCE_ID_LENGTH,
        >::from_sql(value)
        .map(Self)
    }
}

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
    errors, generate_organization_id_of_default_length,
    id_type::LengthId,
};

/// A type for customer_id that can be used for customer ids
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, AsExpression, Hash)]
#[diesel(sql_type = sql_types::Text)]
pub struct OrganizationId(
    LengthId<MAX_ALLOWED_MERCHANT_REFERENCE_ID_LENGTH, MIN_REQUIRED_MERCHANT_REFERENCE_ID_LENGTH>,
);

impl Default for OrganizationId {
    fn default() -> Self {
        generate_organization_id_of_default_length()
    }
}

/// This is to display the `OrganizationId` as OrganizationId(abcd)
impl Debug for OrganizationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("OrganizationId")
            .field(&self.0 .0 .0)
            .finish()
    }
}

impl<DB> Queryable<sql_types::Text, DB> for OrganizationId
where
    DB: Backend,
    Self: FromSql<sql_types::Text, DB>,
{
    type Row = Self;

    fn build(row: Self::Row) -> diesel::deserialize::Result<Self> {
        Ok(row)
    }
}

impl OrganizationId {
    pub(crate) fn new(
        organization_id: LengthId<
            MAX_ALLOWED_MERCHANT_REFERENCE_ID_LENGTH,
            MIN_REQUIRED_MERCHANT_REFERENCE_ID_LENGTH,
        >,
    ) -> Self {
        Self(organization_id)
    }

    /// Get the string representation of customer id
    pub fn get_string_repr(&self) -> &str {
        &self.0 .0 .0
    }

    /// Create a Customer id from string
    pub fn from(input_string: Cow<'static, str>) -> Result<Self, errors::ValidationError> {
        let organization_id = LengthId::from(input_string).change_context(
            errors::ValidationError::IncorrectValueProvided {
                field_name: "customer_id",
            },
        )?;

        Ok(Self(organization_id))
    }
}

impl masking::SerializableSecret for OrganizationId {}

impl<DB> ToSql<sql_types::Text, DB> for OrganizationId
where
    DB: Backend,
    LengthId<MAX_ALLOWED_MERCHANT_REFERENCE_ID_LENGTH, MIN_REQUIRED_MERCHANT_REFERENCE_ID_LENGTH>:
        ToSql<sql_types::Text, DB>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> diesel::serialize::Result {
        self.0.to_sql(out)
    }
}

impl<DB> FromSql<sql_types::Text, DB> for OrganizationId
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

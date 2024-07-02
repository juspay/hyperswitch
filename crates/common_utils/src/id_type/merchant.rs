//! Contains the id type for merchant account
//!
//! Ids for merchant account are derived from the merchant name
//! If there are any special characters, they are removed

use std::{borrow::Cow, fmt::Debug};

use diesel::{
    backend::Backend,
    deserialize::FromSql,
    expression::AsExpression,
    serialize::{Output, ToSql},
    sql_types, Queryable,
};
use error_stack::{Result, ResultExt};
use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::{
    consts::{MAX_ALLOWED_MERCHANT_REFERENCE_ID_LENGTH, MIN_REQUIRED_MERCHANT_REFERENCE_ID_LENGTH},
    errors, generate_ref_id_with_default_length,
    id_type::{AlphaNumericId, LengthId},
    new_type::MerchantName,
};

/// A type for merchant_id that can be used for merchant ids
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, AsExpression)]
#[diesel(sql_type = sql_types::Text)]
pub struct MerchantId(
    LengthId<MAX_ALLOWED_MERCHANT_REFERENCE_ID_LENGTH, MIN_REQUIRED_MERCHANT_REFERENCE_ID_LENGTH>,
);

/// This is to display the `MerchantId` as MerchantId(abcd)
impl Debug for MerchantId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("MerchantId").field(&self.0 .0 .0).finish()
    }
}

impl<DB> Queryable<sql_types::Text, DB> for MerchantId
where
    DB: Backend,
    Self: FromSql<sql_types::Text, DB>,
{
    type Row = Self;

    fn build(row: Self::Row) -> diesel::deserialize::Result<Self> {
        Ok(row)
    }
}

impl Default for MerchantId {
    fn default() -> Self {
        Self(generate_ref_id_with_default_length("mer"))
    }
}

impl MerchantId {
    /// Get the string representation of merchant id
    pub fn get_string_repr(&self) -> &str {
        &self.0 .0 .0
    }

    /// Create a Merchant id from string
    pub fn from(input_string: Cow<'static, str>) -> Result<Self, errors::ValidationError> {
        let length_id = LengthId::from(input_string).change_context(
            errors::ValidationError::IncorrectValueProvided {
                field_name: "merchant_id",
            },
        )?;

        Ok(Self(length_id))
    }

    /// Create a Merchant id from MerchantName
    /// This
    pub fn from_merchant_name(merchant_name: MerchantName) -> Self {
        let merchant_name_string = merchant_name.into_inner();

        let merchant_id = merchant_name_string.trim().to_lowercase().replace(' ', "_");

        // Generate a 3 digit random number so that colission will be less
        let mut rng = rand::thread_rng();
        let randomness: u32 = rng.gen_range(100..1000);

        let merchant_id = format!("{merchant_id}{randomness}");

        let alphanumeric_id = AlphaNumericId::new_unchecked(merchant_id);
        let length_id = LengthId::new_unchecked(alphanumeric_id);

        Self(length_id)
    }
}

impl masking::SerializableSecret for MerchantId {}

impl<DB> ToSql<sql_types::Text, DB> for MerchantId
where
    DB: Backend,
    LengthId<MAX_ALLOWED_MERCHANT_REFERENCE_ID_LENGTH, MIN_REQUIRED_MERCHANT_REFERENCE_ID_LENGTH>:
        ToSql<sql_types::Text, DB>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> diesel::serialize::Result {
        self.0.to_sql(out)
    }
}

impl<DB> FromSql<sql_types::Text, DB> for MerchantId
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

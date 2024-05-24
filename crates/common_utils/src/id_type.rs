//! Common ID types

use std::{
    borrow::Cow,
    fmt::{Debug, Display},
};

mod customer;

pub use customer::CustomerId;
use diesel::{
    backend::Backend,
    deserialize::FromSql,
    expression::AsExpression,
    serialize::{Output, ToSql},
    sql_types,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{fp_utils::when, generate_id_with_default_len};

/// This functions checks for the input string to contain valid characters
/// Returns Some(char) if there are any invalid characters, else None
fn get_invalid_input_character(input_string: Cow<'static, str>) -> Option<char> {
    input_string
        .trim()
        .chars()
        .find(|char| !char.is_ascii_alphanumeric() && !matches!(char, '_' | '-'))
}

#[derive(Debug, PartialEq, Serialize, Clone, Eq)]
/// A type for alphanumeric ids
pub(crate) struct AlphaNumericId(String);

#[derive(Debug, Deserialize, Serialize, Error, Eq, PartialEq)]
#[error("value `{0}` contains invalid character `{1}`")]
/// The error type for alphanumeric id
pub(crate) struct AlphaNumericIdError(String, char);

impl<'de> Deserialize<'de> for AlphaNumericId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let deserialized_string = String::deserialize(deserializer)?;
        Self::from(deserialized_string.into()).map_err(serde::de::Error::custom)
    }
}

impl AlphaNumericId {
    /// Creates a new alphanumeric id from string by applying validation checks
    pub fn from(input_string: Cow<'static, str>) -> Result<Self, AlphaNumericIdError> {
        let invalid_character = get_invalid_input_character(input_string.clone());

        if let Some(invalid_character) = invalid_character {
            Err(AlphaNumericIdError(
                input_string.to_string(),
                invalid_character,
            ))?
        }

        Ok(Self(input_string.to_string()))
    }

    /// Create a new alphanumeric id without any validations
    pub(crate) fn new_unchecked(input_string: String) -> Self {
        Self(input_string)
    }

    /// Generate a new alphanumeric id of default length
    pub(crate) fn new(prefix: &str) -> Self {
        Self(generate_id_with_default_len(prefix))
    }
}

/// A common type of id that can be used for merchant reference ids
#[derive(Debug, Clone, Serialize, PartialEq, Eq, AsExpression)]
#[diesel(sql_type = sql_types::Text)]
pub(crate) struct MerchantReferenceId<const MAX_LENGTH: u8, const MIN_LENGTH: u8>(AlphaNumericId);

/// Error generated from violation of constraints for MerchantReferenceId
#[derive(Debug, Deserialize, Serialize, Error, PartialEq, Eq)]
pub(crate) enum MerchantReferenceIdError<const MAX_LENGTH: u8, const MIN_LENGTH: u8> {
    #[error("the maximum allowed length for this field is {MAX_LENGTH}")]
    /// Maximum length of string violated
    MaxLengthViolated,

    #[error("the minimum required length for this field is {MIN_LENGTH}")]
    /// Minimum length of string violated
    MinLengthViolated,

    #[error("{0}")]
    /// Input contains invalid characters
    AlphanumericIdError(AlphaNumericIdError),
}

impl From<AlphaNumericIdError> for MerchantReferenceIdError<0, 0> {
    fn from(alphanumeric_id_error: AlphaNumericIdError) -> Self {
        Self::AlphanumericIdError(alphanumeric_id_error)
    }
}

impl<const MAX_LENGTH: u8, const MIN_LENGTH: u8> MerchantReferenceId<MAX_LENGTH, MIN_LENGTH> {
    /// Generates new [MerchantReferenceId] from the given input string
    pub fn from(
        input_string: Cow<'static, str>,
    ) -> Result<Self, MerchantReferenceIdError<MAX_LENGTH, MIN_LENGTH>> {
        let trimmed_input_string = input_string.trim().to_string();
        let length_of_input_string = u8::try_from(trimmed_input_string.len())
            .map_err(|_| MerchantReferenceIdError::MaxLengthViolated)?;

        when(length_of_input_string > MAX_LENGTH, || {
            Err(MerchantReferenceIdError::MaxLengthViolated)
        })?;

        when(length_of_input_string < MIN_LENGTH, || {
            Err(MerchantReferenceIdError::MinLengthViolated)
        })?;

        let alphanumeric_id = match AlphaNumericId::from(trimmed_input_string.into()) {
            Ok(valid_alphanumeric_id) => valid_alphanumeric_id,
            Err(error) => Err(MerchantReferenceIdError::AlphanumericIdError(error))?,
        };

        Ok(Self(alphanumeric_id))
    }

    /// Generate a new MerchantRefId of default length with the given prefix
    pub fn new(prefix: &str) -> Self {
        Self(AlphaNumericId::new(prefix))
    }
}

impl<'de, const MAX_LENGTH: u8, const MIN_LENGTH: u8> Deserialize<'de>
    for MerchantReferenceId<MAX_LENGTH, MIN_LENGTH>
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let deserialized_string = String::deserialize(deserializer)?;
        Self::from(deserialized_string.into()).map_err(serde::de::Error::custom)
    }
}

impl<DB, const MAX_LENGTH: u8, const MIN_LENGTH: u8> ToSql<sql_types::Text, DB>
    for MerchantReferenceId<MAX_LENGTH, MIN_LENGTH>
where
    DB: Backend,
    String: ToSql<sql_types::Text, DB>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> diesel::serialize::Result {
        self.0 .0.to_sql(out)
    }
}

impl<const MAX_LENGTH: u8, const MIN_LENGTH: u8> Display
    for MerchantReferenceId<MAX_LENGTH, MIN_LENGTH>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0 .0)
    }
}

impl<DB, const MAX_LENGTH: u8, const MIN_LENGTH: u8> FromSql<sql_types::Text, DB>
    for MerchantReferenceId<MAX_LENGTH, MIN_LENGTH>
where
    DB: Backend,
    String: FromSql<sql_types::Text, DB>,
{
    fn from_sql(value: DB::RawValue<'_>) -> diesel::deserialize::Result<Self> {
        let string_val = String::from_sql(value)?;
        Ok(Self(AlphaNumericId::new_unchecked(string_val)))
    }
}

#[cfg(test)]
mod alphanumeric_id_tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    const VALID_UNDERSCORE_ID_JSON: &str = r#""cus_abcdefghijklmnopqrstuv""#;
    const EXPECTED_VALID_UNDERSCORE_ID: &str = "cus_abcdefghijklmnopqrstuv";

    const VALID_HYPHEN_ID_JSON: &str = r#""cus-abcdefghijklmnopqrstuv""#;
    const VALID_HYPHEN_ID_STRING: &str = "cus-abcdefghijklmnopqrstuv";

    const INVALID_ID_WITH_SPACES: &str = r#""cus abcdefghijklmnopqrstuv""#;
    const INVALID_ID_WITH_EMOJIS: &str = r#""cus_abc🦀""#;

    #[test]
    fn test_id_deserialize_underscore() {
        let parsed_alphanumeric_id =
            serde_json::from_str::<AlphaNumericId>(VALID_UNDERSCORE_ID_JSON);
        let alphanumeric_id = AlphaNumericId::from(EXPECTED_VALID_UNDERSCORE_ID.into()).unwrap();

        assert_eq!(parsed_alphanumeric_id.unwrap(), alphanumeric_id);
    }

    #[test]
    fn test_id_deserialize_hyphen() {
        let parsed_alphanumeric_id = serde_json::from_str::<AlphaNumericId>(VALID_HYPHEN_ID_JSON);
        let alphanumeric_id = AlphaNumericId::from(VALID_HYPHEN_ID_STRING.into()).unwrap();

        assert_eq!(parsed_alphanumeric_id.unwrap(), alphanumeric_id);
    }

    #[test]
    fn test_id_deserialize_with_spaces() {
        let parsed_alphanumeric_id = serde_json::from_str::<AlphaNumericId>(INVALID_ID_WITH_SPACES);

        assert!(parsed_alphanumeric_id.is_err());
    }

    #[test]
    fn test_id_deserialize_with_emojis() {
        let parsed_alphanumeric_id = serde_json::from_str::<AlphaNumericId>(INVALID_ID_WITH_EMOJIS);

        assert!(parsed_alphanumeric_id.is_err());
    }
}

#[cfg(test)]
mod merchant_reference_id_tests {
    use super::*;

    const VALID_REF_ID_JSON: &str = r#""cus_abcdefghijklmnopqrstuv""#;
    const MAX_LENGTH: u8 = 36;
    const MIN_LENGTH: u8 = 6;

    const INVALID_REF_ID_JSON: &str = r#""cus abcdefghijklmnopqrstuv""#;
    const INVALID_REF_ID_LENGTH: &str = r#""cus_abcdefghijklmnopqrstuvwxyzabcdefghij""#;

    #[test]
    fn test_valid_reference_id() {
        let parsed_merchant_reference_id =
            serde_json::from_str::<MerchantReferenceId<MAX_LENGTH, MIN_LENGTH>>(VALID_REF_ID_JSON);

        dbg!(&parsed_merchant_reference_id);

        assert!(parsed_merchant_reference_id.is_ok());
    }

    #[test]
    fn test_invalid_ref_id() {
        let parsed_merchant_reference_id = serde_json::from_str::<
            MerchantReferenceId<MAX_LENGTH, MIN_LENGTH>,
        >(INVALID_REF_ID_JSON);

        assert!(parsed_merchant_reference_id.is_err());
    }

    #[test]
    fn test_invalid_ref_id_error_message() {
        let parsed_merchant_reference_id = serde_json::from_str::<
            MerchantReferenceId<MAX_LENGTH, MIN_LENGTH>,
        >(INVALID_REF_ID_JSON);

        let expected_error_message =
            r#"value `cus abcdefghijklmnopqrstuv` contains invalid character ` `"#.to_string();

        let error_message = parsed_merchant_reference_id
            .err()
            .map(|error| error.to_string());

        assert_eq!(error_message, Some(expected_error_message));
    }

    #[test]
    fn test_invalid_ref_id_length() {
        let parsed_merchant_reference_id = serde_json::from_str::<
            MerchantReferenceId<MAX_LENGTH, MIN_LENGTH>,
        >(INVALID_REF_ID_LENGTH);

        dbg!(&parsed_merchant_reference_id);

        let expected_error_message =
            format!("the maximum allowed length for this field is {MAX_LENGTH}");

        assert!(parsed_merchant_reference_id
            .is_err_and(|error_string| error_string.to_string().eq(&expected_error_message)));
    }

    #[test]
    fn test_invalid_ref_id_length_error_type() {
        let parsed_merchant_reference_id =
            MerchantReferenceId::<MAX_LENGTH, MIN_LENGTH>::from(INVALID_REF_ID_LENGTH.into());

        dbg!(&parsed_merchant_reference_id);

        assert!(
            parsed_merchant_reference_id.is_err_and(|error_type| matches!(
                error_type,
                MerchantReferenceIdError::MaxLengthViolated
            ))
        );
    }
}

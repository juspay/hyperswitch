//! Common ID types

use std::{borrow::Cow, ops::Deref};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::fp_utils::when;

/// This functions checks for the input string to contain valid characters
/// Returns true if the string is valid or else false
fn validate_alphanumeric_id(input_string: Cow<'static, str>) -> bool {
    input_string
        .trim()
        .chars()
        .all(|char| char.is_alphanumeric() || matches!(char, '_' | '-'))
}

#[derive(Debug, PartialEq, Serialize, Clone, Eq)]
/// A type for alphanumeric ids
pub struct AlphaNumericId(String);

#[derive(Debug, Deserialize, Serialize, Error)]
#[error("contains invalid characters, allowed characters are alphanumeric, _ and -")]
/// The error type for alphanumeric id
pub struct AlphaNumericIdError;

impl<'de> Deserialize<'de> for AlphaNumericId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let deserialized_string = String::deserialize(deserializer)?;
        Self::new(deserialized_string.into()).map_err(serde::de::Error::custom)
    }
}

impl AlphaNumericId {
    /// Creates a new alphanumeric id from string
    pub fn new(input_string: Cow<'static, str>) -> Result<Self, AlphaNumericIdError> {
        when(!validate_alphanumeric_id(input_string.clone()), || {
            Err(AlphaNumericIdError)
        })?;

        Ok(Self(input_string.to_string()))
    }

    /// Get the inner value as &str
    pub fn into_inner(&self) -> &str {
        &self.0
    }
}

/// A common type of id that can be used for merchant reference ids in api models
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct MerchantReferenceId<const MAX_LENGTH: u8, const MIN_LENGTH: u8>(AlphaNumericId);

/// Deref can be implemented safely because the type is always valid once it is deserialized
impl<const MAX_LENGTH: u8, const MIN_LENGTH: u8> Deref
    for MerchantReferenceId<MAX_LENGTH, MIN_LENGTH>
{
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.into_inner()
    }
}

/// Error genereted from violation of constraints for MerchantReferenceId
#[derive(Debug, Deserialize, Serialize, Error)]
pub enum MerchantReferenceIdError<const MAX_LENGTH: u8, const MIN_LENGTH: u8> {
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
    fn from(_alphanumeric_id_error: AlphaNumericIdError) -> Self {
        Self::AlphanumericIdError(AlphaNumericIdError)
    }
}

impl<const MAX_LENGTH: u8, const MIN_LENGTH: u8> MerchantReferenceId<MAX_LENGTH, MIN_LENGTH> {
    /// Generates new [MerchantReferenceId] from the given input string
    pub fn new(
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

        let alphanumeric_id = match AlphaNumericId::new(trimmed_input_string.into()) {
            Ok(valid_alphanumeric_id) => valid_alphanumeric_id,
            Err(error) => Err(MerchantReferenceIdError::AlphanumericIdError(error))?,
        };

        Ok(Self(alphanumeric_id))
    }

    /// Get the inner value as &str
    pub fn into_inner(&self) -> &str {
        self.0.into_inner()
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
        Self::new(deserialized_string.into()).map_err(serde::de::Error::custom)
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
        let alphanumeric_id = AlphaNumericId::new(EXPECTED_VALID_UNDERSCORE_ID.into()).unwrap();

        assert_eq!(parsed_alphanumeric_id.unwrap(), alphanumeric_id);
    }

    #[test]
    fn test_id_deserialize_hyphen() {
        let parsed_alphanumeric_id = serde_json::from_str::<AlphaNumericId>(VALID_HYPHEN_ID_JSON);
        let alphanumeric_id = AlphaNumericId::new(VALID_HYPHEN_ID_STRING.into()).unwrap();

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
    fn test_invalid_ref_id_error_type() {
        let parsed_merchant_reference_id =
            MerchantReferenceId::<MAX_LENGTH, MIN_LENGTH>::new(INVALID_REF_ID_JSON.into());

        assert!(
            parsed_merchant_reference_id.is_err_and(|error_type| matches!(
                error_type,
                MerchantReferenceIdError::AlphanumericIdError(AlphaNumericIdError)
            ))
        );
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
            MerchantReferenceId::<MAX_LENGTH, MIN_LENGTH>::new(INVALID_REF_ID_LENGTH.into());

        dbg!(&parsed_merchant_reference_id);

        assert!(
            parsed_merchant_reference_id.is_err_and(|error_type| matches!(
                error_type,
                MerchantReferenceIdError::MaxLengthViolated
            ))
        );
    }
}

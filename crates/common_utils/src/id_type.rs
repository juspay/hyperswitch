//! Common ID types
//! The id type can be used to create specific id types with custom behaviour

mod api_key;
mod client_secret;
mod customer;
#[cfg(feature = "v2")]
mod global_id;
mod merchant;
mod merchant_connector_account;
mod organization;
mod payment;
mod profile;
mod refunds;
mod relay;
mod routing;
mod tenant;

use std::{borrow::Cow, fmt::Debug};

use diesel::{
    backend::Backend,
    deserialize::FromSql,
    expression::AsExpression,
    serialize::{Output, ToSql},
    sql_types,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[cfg(feature = "v2")]
pub use self::global_id::{
    customer::GlobalCustomerId,
    payment::{GlobalAttemptId, GlobalPaymentId},
    payment_methods::{GlobalPaymentMethodId, GlobalPaymentMethodSessionId},
    refunds::GlobalRefundId,
    CellId,
};
pub use self::{
    api_key::ApiKeyId,
    client_secret::ClientSecretId,
    customer::CustomerId,
    merchant::MerchantId,
    merchant_connector_account::MerchantConnectorAccountId,
    organization::OrganizationId,
    payment::{PaymentId, PaymentReferenceId},
    profile::ProfileId,
    refunds::RefundReferenceId,
    relay::RelayId,
    routing::RoutingId,
    tenant::TenantId,
};
use crate::{fp_utils::when, generate_id_with_default_len};

#[inline]
fn is_valid_id_character(input_char: char) -> bool {
    input_char.is_ascii_alphanumeric() || matches!(input_char, '_' | '-')
}

/// This functions checks for the input string to contain valid characters
/// Returns Some(char) if there are any invalid characters, else None
fn get_invalid_input_character(input_string: Cow<'static, str>) -> Option<char> {
    input_string
        .trim()
        .chars()
        .find(|&char| !is_valid_id_character(char))
}

#[derive(Debug, PartialEq, Hash, Serialize, Clone, Eq)]
/// A type for alphanumeric ids
pub(crate) struct AlphaNumericId(String);

#[derive(Debug, Deserialize, Hash, Serialize, Error, Eq, PartialEq)]
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

/// A common type of id that can be used for reference ids with length constraint
#[derive(Debug, Clone, Serialize, Hash, PartialEq, Eq, AsExpression)]
#[diesel(sql_type = sql_types::Text)]
pub(crate) struct LengthId<const MAX_LENGTH: u8, const MIN_LENGTH: u8>(AlphaNumericId);

/// Error generated from violation of constraints for MerchantReferenceId
#[derive(Debug, Error, PartialEq, Eq)]
pub(crate) enum LengthIdError {
    #[error("the maximum allowed length for this field is {0}")]
    /// Maximum length of string violated
    MaxLengthViolated(u8),

    #[error("the minimum required length for this field is {0}")]
    /// Minimum length of string violated
    MinLengthViolated(u8),

    #[error("{0}")]
    /// Input contains invalid characters
    AlphanumericIdError(AlphaNumericIdError),
}

impl From<AlphaNumericIdError> for LengthIdError {
    fn from(alphanumeric_id_error: AlphaNumericIdError) -> Self {
        Self::AlphanumericIdError(alphanumeric_id_error)
    }
}

impl<const MAX_LENGTH: u8, const MIN_LENGTH: u8> LengthId<MAX_LENGTH, MIN_LENGTH> {
    /// Generates new [MerchantReferenceId] from the given input string
    pub fn from(input_string: Cow<'static, str>) -> Result<Self, LengthIdError> {
        let trimmed_input_string = input_string.trim().to_string();
        let length_of_input_string = u8::try_from(trimmed_input_string.len())
            .map_err(|_| LengthIdError::MaxLengthViolated(MAX_LENGTH))?;

        when(length_of_input_string > MAX_LENGTH, || {
            Err(LengthIdError::MaxLengthViolated(MAX_LENGTH))
        })?;

        when(length_of_input_string < MIN_LENGTH, || {
            Err(LengthIdError::MinLengthViolated(MIN_LENGTH))
        })?;

        let alphanumeric_id = match AlphaNumericId::from(trimmed_input_string.into()) {
            Ok(valid_alphanumeric_id) => valid_alphanumeric_id,
            Err(error) => Err(LengthIdError::AlphanumericIdError(error))?,
        };

        Ok(Self(alphanumeric_id))
    }

    /// Generate a new MerchantRefId of default length with the given prefix
    pub fn new(prefix: &str) -> Self {
        Self(AlphaNumericId::new(prefix))
    }

    /// Use this function only if you are sure that the length is within the range
    pub(crate) fn new_unchecked(alphanumeric_id: AlphaNumericId) -> Self {
        Self(alphanumeric_id)
    }

    #[cfg(feature = "v2")]
    /// Create a new LengthId from aplhanumeric id
    pub(crate) fn from_alphanumeric_id(
        alphanumeric_id: AlphaNumericId,
    ) -> Result<Self, LengthIdError> {
        let length_of_input_string = alphanumeric_id.0.len();
        let length_of_input_string = u8::try_from(length_of_input_string)
            .map_err(|_| LengthIdError::MaxLengthViolated(MAX_LENGTH))?;

        when(length_of_input_string > MAX_LENGTH, || {
            Err(LengthIdError::MaxLengthViolated(MAX_LENGTH))
        })?;

        when(length_of_input_string < MIN_LENGTH, || {
            Err(LengthIdError::MinLengthViolated(MIN_LENGTH))
        })?;

        Ok(Self(alphanumeric_id))
    }
}

impl<'de, const MAX_LENGTH: u8, const MIN_LENGTH: u8> Deserialize<'de>
    for LengthId<MAX_LENGTH, MIN_LENGTH>
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
    for LengthId<MAX_LENGTH, MIN_LENGTH>
where
    DB: Backend,
    String: ToSql<sql_types::Text, DB>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> diesel::serialize::Result {
        self.0 .0.to_sql(out)
    }
}

impl<DB, const MAX_LENGTH: u8, const MIN_LENGTH: u8> FromSql<sql_types::Text, DB>
    for LengthId<MAX_LENGTH, MIN_LENGTH>
where
    DB: Backend,
    String: FromSql<sql_types::Text, DB>,
{
    fn from_sql(value: DB::RawValue<'_>) -> diesel::deserialize::Result<Self> {
        let string_val = String::from_sql(value)?;
        Ok(Self(AlphaNumericId::new_unchecked(string_val)))
    }
}

/// An interface to generate object identifiers.
pub trait GenerateId {
    /// Generates a random object identifier.
    fn generate() -> Self;
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
            serde_json::from_str::<LengthId<MAX_LENGTH, MIN_LENGTH>>(VALID_REF_ID_JSON);

        dbg!(&parsed_merchant_reference_id);

        assert!(parsed_merchant_reference_id.is_ok());
    }

    #[test]
    fn test_invalid_ref_id() {
        let parsed_merchant_reference_id =
            serde_json::from_str::<LengthId<MAX_LENGTH, MIN_LENGTH>>(INVALID_REF_ID_JSON);

        assert!(parsed_merchant_reference_id.is_err());
    }

    #[test]
    fn test_invalid_ref_id_error_message() {
        let parsed_merchant_reference_id =
            serde_json::from_str::<LengthId<MAX_LENGTH, MIN_LENGTH>>(INVALID_REF_ID_JSON);

        let expected_error_message =
            r#"value `cus abcdefghijklmnopqrstuv` contains invalid character ` `"#.to_string();

        let error_message = parsed_merchant_reference_id
            .err()
            .map(|error| error.to_string());

        assert_eq!(error_message, Some(expected_error_message));
    }

    #[test]
    fn test_invalid_ref_id_length() {
        let parsed_merchant_reference_id =
            serde_json::from_str::<LengthId<MAX_LENGTH, MIN_LENGTH>>(INVALID_REF_ID_LENGTH);

        dbg!(&parsed_merchant_reference_id);

        let expected_error_message =
            format!("the maximum allowed length for this field is {MAX_LENGTH}");

        assert!(parsed_merchant_reference_id
            .is_err_and(|error_string| error_string.to_string().eq(&expected_error_message)));
    }

    #[test]
    fn test_invalid_ref_id_length_error_type() {
        let parsed_merchant_reference_id =
            LengthId::<MAX_LENGTH, MIN_LENGTH>::from(INVALID_REF_ID_LENGTH.into());

        dbg!(&parsed_merchant_reference_id);

        assert!(
            parsed_merchant_reference_id.is_err_and(|error_type| matches!(
                error_type,
                LengthIdError::MaxLengthViolated(MAX_LENGTH)
            ))
        );
    }
}

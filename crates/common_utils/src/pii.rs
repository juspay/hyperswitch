//! Personal Identifiable Information protection.

use std::{convert::AsRef, fmt, ops, str::FromStr};

use diesel::{
    backend::Backend,
    deserialize,
    deserialize::FromSql,
    prelude::*,
    serialize::{Output, ToSql},
    sql_types, AsExpression,
};
use error_stack::{IntoReport, ResultExt};
use masking::{ExposeInterface, Secret, Strategy, WithType};
#[cfg(feature = "logs")]
use router_env::logger;

use crate::{
    crypto::Encryptable,
    errors::{self, ValidationError},
    validation::{validate_email, validate_phone_number},
};

/// A string constant representing a redacted or masked value.
pub const REDACTED: &str = "Redacted";

/// Type alias for serde_json value which has Secret Information
pub type SecretSerdeValue = Secret<serde_json::Value>;

/// Strategy for masking a PhoneNumber
#[derive(Debug)]
pub enum PhoneNumberStrategy {}

/// Phone Number
#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(try_from = "String")]
pub struct PhoneNumber(Secret<String, PhoneNumberStrategy>);

impl<T> Strategy<T> for PhoneNumberStrategy
where
    T: AsRef<str> + std::fmt::Debug,
{
        /// Formats the value `val` by masking everything but the last 4 digits and writes the result to the provided formatter `f`.
    fn fmt(val: &T, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let val_str: &str = val.as_ref();

        if let Some(val_str) = val_str.get(val_str.len() - 4..) {
            // masks everything but the last 4 digits
            write!(f, "{}{}", "*".repeat(val_str.len() - 4), val_str)
        } else {
            #[cfg(feature = "logs")]
            logger::error!("Invalid phone number: {val_str}");
            WithType::fmt(val, f)
        }
    }
}

impl FromStr for PhoneNumber {
    type Err = error_stack::Report<ValidationError>;
        /// Parses a phone number string and returns a Result containing a new instance of the PhoneNumber struct if the phone number is valid, or an error if the phone number is invalid.
    fn from_str(phone_number: &str) -> Result<Self, Self::Err> {
        validate_phone_number(phone_number)?;
        let secret = Secret::<String, PhoneNumberStrategy>::new(phone_number.to_string());
        Ok(Self(secret))
    }
}

impl TryFrom<String> for PhoneNumber {
    type Error = error_stack::Report<errors::ParsingError>;

        /// Attempts to create a new instance of the current type from the given String value.
    /// If successful, returns a Result containing the newly created instance,
    /// otherwise returns a Result containing the error encountered during the creation process.
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::from_str(&value).change_context(errors::ParsingError::PhoneNumberParsingError)
    }
}

impl ops::Deref for PhoneNumber {
    type Target = Secret<String, PhoneNumberStrategy>;

        /// This method returns a reference to the value that the `Deref` trait dereferences to.
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ops::DerefMut for PhoneNumber {
        /// This method returns a mutable reference to the target type of the smart pointer.
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<DB> Queryable<diesel::sql_types::Text, DB> for PhoneNumber
where
    DB: Backend,
    Self: FromSql<sql_types::Text, DB>,
{
    type Row = Self;

        /// This method takes a row and returns a Result containing the deserialized data of type Self.
    fn build(row: Self::Row) -> deserialize::Result<Self> {
        Ok(row)
    }
}

impl<DB> FromSql<sql_types::Text, DB> for PhoneNumber
where
    DB: Backend,
    String: FromSql<sql_types::Text, DB>,
{
        /// Converts a raw value from the database into a deserialized result of Self.
    fn from_sql(bytes: DB::RawValue<'_>) -> deserialize::Result<Self> {
        let val = String::from_sql(bytes)?;
        Ok(Self::from_str(val.as_str())?)
    }
}

impl<DB> ToSql<sql_types::Text, DB> for PhoneNumber
where
    DB: Backend,
    String: ToSql<sql_types::Text, DB>,
{
        /// Converts the value to its SQL representation and writes it to the provided output.
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> diesel::serialize::Result {
        self.0.to_sql(out)
    }
}

/*
/// Phone number
#[derive(Debug)]
pub struct PhoneNumber;

impl<T> Strategy<T> for PhoneNumber
where
    T: AsRef<str>,
{
    fn fmt(val: &T, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let val_str: &str = val.as_ref();

        if val_str.len() < 10 || val_str.len() > 12 {
            return WithType::fmt(val, f);
        }

        write!(
            f,
            "{}{}{}",
            &val_str[..2],
            "*".repeat(val_str.len() - 5),
            &val_str[(val_str.len() - 3)..]
        )
    }
}
*/

/// Strategy for Encryption
#[derive(Debug)]
pub enum EncryptionStratergy {}

impl<T> Strategy<T> for EncryptionStratergy
where
    T: AsRef<[u8]>,
{
        /// Formats the given value by writing the length of its reference as the number of bytes, enclosed in an encrypted message, to the provided formatter.
    fn fmt(value: &T, fmt: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(fmt, "*** Encrypted {} of bytes ***", value.as_ref().len())
    }
}

/// Client secret
#[derive(Debug)]
pub enum ClientSecret {}

impl<T> Strategy<T> for ClientSecret
where
    T: AsRef<str>,
{
        /// Formats a value of type T by replacing part of the value with asterisks if it represents a client secret in the format "pay_{client_id}_secret_*".
    fn fmt(val: &T, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let val_str: &str = val.as_ref();

        let client_secret_segments: Vec<&str> = val_str.split('_').collect();

        if client_secret_segments.len() != 4
            || !client_secret_segments.contains(&"pay")
            || !client_secret_segments.contains(&"secret")
        {
            return WithType::fmt(val, f);
        }

        if let Some((client_secret_segments_0, client_secret_segments_1)) = client_secret_segments
            .first()
            .zip(client_secret_segments.get(1))
        {
            write!(
                f,
                "{}_{}_{}",
                client_secret_segments_0,
                client_secret_segments_1,
                "*".repeat(
                    val_str.len()
                        - (client_secret_segments_0.len() + client_secret_segments_1.len() + 2)
                )
            )
        } else {
            #[cfg(feature = "logs")]
            logger::error!("Invalid client secret: {val_str}");
            WithType::fmt(val, f)
        }
    }
}

/// Strategy for masking Email
#[derive(Debug)]
pub enum EmailStrategy {}

impl<T> Strategy<T> for EmailStrategy
where
    T: AsRef<str> + std::fmt::Debug,
{
        /// Formats the value `val` using the given formatter `f`. If the value is a string containing an '@' symbol, it replaces the part before the '@' with '*' characters of the same length, and writes the result to the formatter. If the value does not contain an '@' symbol, it delegates to the `fmt` method of the `WithType` trait.
    fn fmt(val: &T, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let val_str: &str = val.as_ref();
        match val_str.split_once('@') {
            Some((a, b)) => write!(f, "{}@{}", "*".repeat(a.len()), b),
            None => WithType::fmt(val, f),
        }
    }
}
/// Email address
#[derive(
    serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq, Default, AsExpression,
)]
#[diesel(sql_type = diesel::sql_types::Text)]
#[serde(try_from = "String")]
pub struct Email(Secret<String, EmailStrategy>);

impl From<Encryptable<Secret<String, EmailStrategy>>> for Email {
        /// Converts an `Encryptable` with a wrapped `Secret<String, EmailStrategy>` into the inner value of the `Encryptable`
    fn from(item: Encryptable<Secret<String, EmailStrategy>>) -> Self {
        Self(item.into_inner())
    }
}

impl ExposeInterface<Secret<String, EmailStrategy>> for Email {
        /// This method takes ownership of the object and returns the internal data as a Secret type with a String payload and EmailStrategy access control strategy.
    fn expose(self) -> Secret<String, EmailStrategy> {
        self.0
    }
}

impl TryFrom<String> for Email {
    type Error = error_stack::Report<errors::ParsingError>;

        /// Attempts to create a new instance of the current type from the provided String value.
    /// Returns a Result with the new instance on success, or an error of the associated error type on failure.
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::from_str(&value).change_context(errors::ParsingError::EmailParsingError)
    }
}

impl ops::Deref for Email {
    type Target = Secret<String, EmailStrategy>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ops::DerefMut for Email {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<DB> Queryable<diesel::sql_types::Text, DB> for Email
where
    DB: Backend,
    Self: FromSql<sql_types::Text, DB>,
{
    type Row = Self;

    fn build(row: Self::Row) -> deserialize::Result<Self> {
        Ok(row)
    }
}

impl<DB> FromSql<sql_types::Text, DB> for Email
where
    DB: Backend,
    String: FromSql<sql_types::Text, DB>,
{
    fn from_sql(bytes: DB::RawValue<'_>) -> deserialize::Result<Self> {
        let val = String::from_sql(bytes)?;
        Ok(Self::from_str(val.as_str())?)
    }
}

impl<DB> ToSql<sql_types::Text, DB> for Email
where
    DB: Backend,
    String: ToSql<sql_types::Text, DB>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> diesel::serialize::Result {
        self.0.to_sql(out)
    }
}

impl FromStr for Email {
    type Err = error_stack::Report<ValidationError>;
        /// Parses a string into an Email struct, returning a Result.
    /// If the email is a redacted string, it creates a Secret with the email as its value and returns it as Ok.
    /// Otherwise, it validates the email using the validate_email function and returns either an Email containing the validated email as Ok, or a ValidationError containing an error message as Err.
    fn from_str(email: &str) -> Result<Self, Self::Err> {
        if email.eq(REDACTED) {
            return Ok(Self(Secret::new(email.to_string())));
        }
        match validate_email(email) {
            Ok(_) => {
                let secret = Secret::<String, EmailStrategy>::new(email.to_string());
                Ok(Self(secret))
            }
            Err(_) => Err(ValidationError::InvalidValue {
                message: "Invalid email address format".into(),
            })
            .into_report(),
        }
    }
}

/// IP address
#[derive(Debug)]
pub enum IpAddress {}

impl<T> Strategy<T> for IpAddress
where
    T: AsRef<str>,
{
        /// Formats the given IP address value by replacing the last three segments with "**" if the IP address consists of four segments separated by '.'. If the IP address does not meet this criteria, it falls back to formatting it with the default behavior.
    fn fmt(val: &T, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let val_str: &str = val.as_ref();
        let segments: Vec<&str> = val_str.split('.').collect();

        if segments.len() != 4 {
            return WithType::fmt(val, f);
        }

        for seg in segments.iter() {
            if seg.is_empty() || seg.len() > 3 {
                return WithType::fmt(val, f);
            }
        }

        if let Some(segments) = segments.first() {
            write!(f, "{}.**.**.**", segments)
        } else {
            #[cfg(feature = "logs")]
            logger::error!("Invalid IP address: {val_str}");
            WithType::fmt(val, f)
        }
    }
}

/// Strategy for masking UPI VPA's

#[derive(Debug)]
pub enum UpiVpaMaskingStrategy {}

impl<T> Strategy<T> for UpiVpaMaskingStrategy
where
    T: AsRef<str> + std::fmt::Debug,
{
        /// Formats the VPA (Virtual Payment Address) by masking the user identifier with asterisks,
    /// and leaving the bank or PSP unchanged. If the VPA does not contain '@' symbol, it delegates
    /// the formatting to the `WithType` implementation for the underlying type `T`.
    fn fmt(val: &T, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let vpa_str: &str = val.as_ref();
        if let Some((user_identifier, bank_or_psp)) = vpa_str.split_once('@') {
            let masked_user_identifier = "*".repeat(user_identifier.len());
            write!(f, "{masked_user_identifier}@{bank_or_psp}")
        } else {
            WithType::fmt(val, f)
        }
    }
}

#[cfg(test)]
mod pii_masking_strategy_tests {
    use std::str::FromStr;

    use masking::{ExposeInterface, Secret};

    use super::{ClientSecret, Email, IpAddress, UpiVpaMaskingStrategy};
    use crate::pii::{EmailStrategy, REDACTED};

    /*
    #[test]
    fn test_valid_phone_number_masking() {
        let secret: Secret<String, PhoneNumber> = Secret::new("9922992299".to_string());
        assert_eq!("99*****299", format!("{}", secret));
    }

    #[test]
    fn test_invalid_phone_number_masking() {
        let secret: Secret<String, PhoneNumber> = Secret::new("99229922".to_string());
        assert_eq!("*** alloc::string::String ***", format!("{}", secret));

        let secret: Secret<String, PhoneNumber> = Secret::new("9922992299229922".to_string());
        assert_eq!("*** alloc::string::String ***", format!("{}", secret));
    }
    */

    #[test]
        /// Tests the valid email masking by creating a Secret with email strategy, then asserts that the email is masked correctly.
    fn test_valid_email_masking() {
        let secret: Secret<String, EmailStrategy> = Secret::new("example@test.com".to_string());
        assert_eq!("*******@test.com", format!("{secret:?}"));

        let secret: Secret<String, EmailStrategy> = Secret::new("username@gmail.com".to_string());
        assert_eq!("********@gmail.com", format!("{secret:?}"));
    }

    #[test]
        /// Tests the invalid email masking functionality by creating a secret with invalid email addresses
    fn test_invalid_email_masking() {
        let secret: Secret<String, EmailStrategy> = Secret::new("myemailgmail.com".to_string());
        assert_eq!("*** alloc::string::String ***", format!("{secret:?}"));

        let secret: Secret<String, EmailStrategy> = Secret::new("myemail$gmail.com".to_string());
        assert_eq!("*** alloc::string::String ***", format!("{secret:?}"));
    }

    #[test]
        /// This method is used to test the validity of a newtype email address. It creates a new Email instance using the `from_str` method and checks if it is a valid email address by asserting that the result of `is_ok()` is true.
    fn test_valid_newtype_email() {
        let email_check = Email::from_str("example@abc.com");
        assert!(email_check.is_ok());
    }

    #[test]
        /// Test function to check if an invalid newtype email address is correctly identified as an error.
    fn test_invalid_newtype_email() {
        let email_check = Email::from_str("example@abc@com");
        assert!(email_check.is_err());
    }

    #[test]
        /// This method tests the creation and extraction of a redacted email address. It creates an Email instance from a redacted string, asserts that the creation is successful, and then extracts and compares the secret value with the original redacted string.
    fn test_redacted_email() {
        let email_result = Email::from_str(REDACTED);
        assert!(email_result.is_ok());
        if let Ok(email) = email_result {
            let secret_value = email.0.expose();
            assert_eq!(secret_value.as_str(), REDACTED);
        }
    }

    #[test]
        /// This method tests the masking of a valid IP address. It creates a Secret object with a string containing an IP address, then formats the object to mask the IP address and asserts that the masked IP address matches the expected value.
    fn test_valid_ip_addr_masking() {
        let secret: Secret<String, IpAddress> = Secret::new("123.23.1.78".to_string());
        assert_eq!("123.**.**.**", format!("{secret:?}"));
    }

    #[test]
        /// This method tests the masking of invalid IP addresses in a Secret data structure
    fn test_invalid_ip_addr_masking() {
        let secret: Secret<String, IpAddress> = Secret::new("123.4.56".to_string());
        assert_eq!("*** alloc::string::String ***", format!("{secret:?}"));

        let secret: Secret<String, IpAddress> = Secret::new("123.4567.12.4".to_string());
        assert_eq!("*** alloc::string::String ***", format!("{secret:?}"));

        let secret: Secret<String, IpAddress> = Secret::new("123..4.56".to_string());
        assert_eq!("*** alloc::string::String ***", format!("{secret:?}"));
    }

    #[test]
        /// This method tests the masking of a valid client secret by creating a Secret instance with a client secret string and then comparing the masked output with the expected masked value.
    fn test_valid_client_secret_masking() {
        let secret: Secret<String, ClientSecret> =
            Secret::new("pay_uszFB2QGe9MmLY65ojhT_secret_tLjTz9tAQxUVEFqfmOIP".to_string());
        assert_eq!(
            "pay_uszFB2QGe9MmLY65ojhT_***************************",
            format!("{secret:?}")
        );
    }

    #[test]
        /// Tests the invalid client secret masking by creating a new Secret with a string value and an IP Address type, then asserts that the formatted string representation of the secret starts with "*** alloc::string::String ***".
    fn test_invalid_client_secret_masking() {
        let secret: Secret<String, IpAddress> =
            Secret::new("pay_uszFB2QGe9MmLY65ojhT_secret".to_string());
        assert_eq!("*** alloc::string::String ***", format!("{secret:?}"));
    }

    #[test]
        /// This method tests the default masking behavior for a valid phone number.
    fn test_valid_phone_number_default_masking() {
        let secret: Secret<String> = Secret::new("+40712345678".to_string());
        assert_eq!("*** alloc::string::String ***", format!("{secret:?}"));
    }

    #[test]
        /// This method is used to test the validity of UPI (Unified Payments Interface) VPA (Virtual Payment Address) masking strategy. It creates a Secret object with a UPI VPA masking strategy and asserts that the masked value is equal to the expected value.
    fn test_valid_upi_vpa_masking() {
        let secret: Secret<String, UpiVpaMaskingStrategy> = Secret::new("my_name@upi".to_string());
        assert_eq!("*******@upi", format!("{secret:?}"));
    }

    #[test]
        /// This function tests the invalid UPI VPA masking by creating a new Secret with a UPI VPA masking strategy and
    /// checking if the formatted string representation of the Secret matches the expected value.
    fn test_invalid_upi_vpa_masking() {
        let secret: Secret<String, UpiVpaMaskingStrategy> = Secret::new("my_name_upi".to_string());
        assert_eq!("*** alloc::string::String ***", format!("{secret:?}"));
    }
}

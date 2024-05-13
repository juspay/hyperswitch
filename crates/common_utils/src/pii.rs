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
use error_stack::ResultExt;
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
    T: AsRef<str> + fmt::Debug,
{
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
    fn from_str(phone_number: &str) -> Result<Self, Self::Err> {
        validate_phone_number(phone_number)?;
        let secret = Secret::<String, PhoneNumberStrategy>::new(phone_number.to_string());
        Ok(Self(secret))
    }
}

impl TryFrom<String> for PhoneNumber {
    type Error = error_stack::Report<errors::ParsingError>;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::from_str(&value).change_context(errors::ParsingError::PhoneNumberParsingError)
    }
}

impl ops::Deref for PhoneNumber {
    type Target = Secret<String, PhoneNumberStrategy>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ops::DerefMut for PhoneNumber {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<DB> Queryable<sql_types::Text, DB> for PhoneNumber
where
    DB: Backend,
    Self: FromSql<sql_types::Text, DB>,
{
    type Row = Self;

    fn build(row: Self::Row) -> deserialize::Result<Self> {
        Ok(row)
    }
}

impl<DB> FromSql<sql_types::Text, DB> for PhoneNumber
where
    DB: Backend,
    String: FromSql<sql_types::Text, DB>,
{
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
pub enum EncryptionStrategy {}

impl<T> Strategy<T> for EncryptionStrategy
where
    T: AsRef<[u8]>,
{
    fn fmt(value: &T, fmt: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            fmt,
            "*** Encrypted data of length {} bytes ***",
            value.as_ref().len()
        )
    }
}

/// Client secret
#[derive(Debug)]
pub enum ClientSecret {}

impl<T> Strategy<T> for ClientSecret
where
    T: AsRef<str>,
{
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
    T: AsRef<str> + fmt::Debug,
{
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
#[diesel(sql_type = sql_types::Text)]
#[serde(try_from = "String")]
pub struct Email(Secret<String, EmailStrategy>);

impl From<Encryptable<Secret<String, EmailStrategy>>> for Email {
    fn from(item: Encryptable<Secret<String, EmailStrategy>>) -> Self {
        Self(item.into_inner())
    }
}

impl ExposeInterface<Secret<String, EmailStrategy>> for Email {
    fn expose(self) -> Secret<String, EmailStrategy> {
        self.0
    }
}

impl TryFrom<String> for Email {
    type Error = error_stack::Report<errors::ParsingError>;

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

impl<DB> Queryable<sql_types::Text, DB> for Email
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
            }
            .into()),
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
    T: AsRef<str> + fmt::Debug,
{
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
    fn test_valid_email_masking() {
        let secret: Secret<String, EmailStrategy> = Secret::new("example@test.com".to_string());
        assert_eq!("*******@test.com", format!("{secret:?}"));

        let secret: Secret<String, EmailStrategy> = Secret::new("username@gmail.com".to_string());
        assert_eq!("********@gmail.com", format!("{secret:?}"));
    }

    #[test]
    fn test_invalid_email_masking() {
        let secret: Secret<String, EmailStrategy> = Secret::new("myemailgmail.com".to_string());
        assert_eq!("*** alloc::string::String ***", format!("{secret:?}"));

        let secret: Secret<String, EmailStrategy> = Secret::new("myemail$gmail.com".to_string());
        assert_eq!("*** alloc::string::String ***", format!("{secret:?}"));
    }

    #[test]
    fn test_valid_newtype_email() {
        let email_check = Email::from_str("example@abc.com");
        assert!(email_check.is_ok());
    }

    #[test]
    fn test_invalid_newtype_email() {
        let email_check = Email::from_str("example@abc@com");
        assert!(email_check.is_err());
    }

    #[test]
    fn test_redacted_email() {
        let email_result = Email::from_str(REDACTED);
        assert!(email_result.is_ok());
        if let Ok(email) = email_result {
            let secret_value = email.0.expose();
            assert_eq!(secret_value.as_str(), REDACTED);
        }
    }

    #[test]
    fn test_valid_ip_addr_masking() {
        let secret: Secret<String, IpAddress> = Secret::new("123.23.1.78".to_string());
        assert_eq!("123.**.**.**", format!("{secret:?}"));
    }

    #[test]
    fn test_invalid_ip_addr_masking() {
        let secret: Secret<String, IpAddress> = Secret::new("123.4.56".to_string());
        assert_eq!("*** alloc::string::String ***", format!("{secret:?}"));

        let secret: Secret<String, IpAddress> = Secret::new("123.4567.12.4".to_string());
        assert_eq!("*** alloc::string::String ***", format!("{secret:?}"));

        let secret: Secret<String, IpAddress> = Secret::new("123..4.56".to_string());
        assert_eq!("*** alloc::string::String ***", format!("{secret:?}"));
    }

    #[test]
    fn test_valid_client_secret_masking() {
        let secret: Secret<String, ClientSecret> =
            Secret::new("pay_uszFB2QGe9MmLY65ojhT_secret_tLjTz9tAQxUVEFqfmOIP".to_string());
        assert_eq!(
            "pay_uszFB2QGe9MmLY65ojhT_***************************",
            format!("{secret:?}")
        );
    }

    #[test]
    fn test_invalid_client_secret_masking() {
        let secret: Secret<String, IpAddress> =
            Secret::new("pay_uszFB2QGe9MmLY65ojhT_secret".to_string());
        assert_eq!("*** alloc::string::String ***", format!("{secret:?}"));
    }

    #[test]
    fn test_valid_phone_number_default_masking() {
        let secret: Secret<String> = Secret::new("+40712345678".to_string());
        assert_eq!("*** alloc::string::String ***", format!("{secret:?}"));
    }

    #[test]
    fn test_valid_upi_vpa_masking() {
        let secret: Secret<String, UpiVpaMaskingStrategy> = Secret::new("my_name@upi".to_string());
        assert_eq!("*******@upi", format!("{secret:?}"));
    }

    #[test]
    fn test_invalid_upi_vpa_masking() {
        let secret: Secret<String, UpiVpaMaskingStrategy> = Secret::new("my_name_upi".to_string());
        assert_eq!("*** alloc::string::String ***", format!("{secret:?}"));
    }
}

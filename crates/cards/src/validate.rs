use std::{fmt, ops::Deref, str::FromStr};

use masking::{PeekInterface, Strategy, StrongSecret, WithType};
#[cfg(not(target_arch = "wasm32"))]
use router_env::logger;
use serde::{Deserialize, Deserializer, Serialize};
use thiserror::Error;

#[derive(Debug, Deserialize, Serialize, Error)]
#[error("not a valid credit card number")]
pub struct CCValError;

impl From<core::convert::Infallible> for CCValError {
        /// This method creates a new instance of Self, which is the type it is being called on, without taking any argument.
    fn from(_: core::convert::Infallible) -> Self {
        Self
    }
}

/// Card number
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct CardNumber(StrongSecret<String, CardNumberStrategy>);

impl CardNumber {
        /// Returns the International Securities Identification Number (ISIN) code of the card.
    /// The ISIN is a 12-character alphanumeric code that uniquely identifies a specific securities issue.
    pub fn get_card_isin(self) -> String {
        self.0.peek().chars().take(6).collect::<String>()
    }

        /// Returns the first 8 characters of the peeked string from the given input as a new String.
    pub fn get_extended_card_bin(self) -> String {
        self.0.peek().chars().take(8).collect::<String>()
    }
        /// Retrieves the card number as a String.
    pub fn get_card_no(self) -> String {
        self.0.peek().chars().collect::<String>()
    }
        /// This method returns the last 4 characters of the string it is called on.
    pub fn get_last4(self) -> String {
        self.0
            .peek()
            .chars()
            .rev()
            .take(4)
            .collect::<String>()
            .chars()
            .rev()
            .collect::<String>()
    }
        /// This method returns the first 8 characters of the peeked value from the internal buffer as a String,
    /// representing the extended BIN (Bank Identification Number) of a card.
    pub fn get_card_extended_bin(self) -> String {
            self.0.peek().chars().take(8).collect::<String>()
    }
}

impl FromStr for CardNumber {
    type Err = CCValError;

        /// Parses a string to create a CreditCard object, returning a Result.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match luhn::valid(s) {
            true => {
                let cc_no_whitespace: String = s.split_whitespace().collect();
                Ok(Self(StrongSecret::from_str(&cc_no_whitespace)?))
            }
            false => Err(CCValError),
        }
    }
}

impl TryFrom<String> for CardNumber {
    type Error = CCValError;

        /// Attempts to create a value of the implementing type from a String, returning a Result.
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::from_str(&value)
    }
}

impl Deref for CardNumber {
    type Target = StrongSecret<String, CardNumberStrategy>;

        /// This method returns a reference to the strong secret value containing a string and a card number strategy.
    fn deref(&self) -> &StrongSecret<String, CardNumberStrategy> {
        &self.0
    }
}

impl<'de> Deserialize<'de> for CardNumber {
        /// Deserializes a string from the given Deserializer and constructs a new instance of Self.
    /// 
    /// # Arguments
    /// 
    /// * `d` - A Deserializer that provides the string to be deserialized.
    /// 
    /// # Returns
    /// 
    /// A Result containing either the deserialized instance of Self or an error if deserialization fails.
    /// 
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        Self::from_str(&s).map_err(serde::de::Error::custom)
    }
}

pub enum CardNumberStrategy {}

impl<T> Strategy<T> for CardNumberStrategy
where
    T: AsRef<str>,
{
        /// Formats the given value `val` using the provided formatter `f`. If the length of the value is less than 15 or greater than 19, it delegates the formatting to the `WithType` trait. Otherwise, it truncates the value to 6 characters and replaces the remaining characters with asterisks before writing it to the formatter. If the target architecture is not wasm32, it logs an error message indicating an invalid card number. Returns a `fmt::Result` indicating success or failure of the formatting operation.
    fn fmt(val: &T, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let val_str: &str = val.as_ref();

        if val_str.len() < 15 || val_str.len() > 19 {
            return WithType::fmt(val, f);
        }

        if let Some(value) = val_str.get(..6) {
            write!(f, "{}{}", value, "*".repeat(val_str.len() - 6))
        } else {
            #[cfg(not(target_arch = "wasm32"))]
            logger::error!("Invalid card number {val_str}");
            WithType::fmt(val, f)
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use masking::Secret;

    use super::*;

    #[test]
        /// This method checks if the given card number is valid by converting it into a CardNumber type and comparing it with a StrongSecret type created from the same string representation. It uses the assert_eq macro to compare the two types and asserts that they are equal. 
    fn valid_card_number() {
        let s = "371449635398431";
        assert_eq!(
            CardNumber::from_str(s).unwrap(),
            CardNumber(StrongSecret::from_str(s).unwrap())
        );
    }

    #[test]
        /// This method tests whether a given card number is invalid by attempting to convert it to a CardNumber object and checking if it returns an error, indicating that it is not a valid credit card number.
    fn invalid_card_number() {
        let s = "371446431";
        assert_eq!(
            CardNumber::from_str(s).unwrap_err().to_string(),
            "not a valid credit card number".to_string()
        );
    }

    #[test]
        /// This method removes whitespace from a card number string and asserts that it is correctly converted to a masked format where all but the first six and last four digits are replaced with asterisks.
    fn card_number_no_whitespace() {
        let s = "3714    4963  5398 431";
        assert_eq!(
            CardNumber::from_str(s).unwrap().to_string(),
            "371449*********"
        );
    }

    #[test]
        /// This method is used to test the masking of a valid card number. It creates a new Secret with a card number strategy, initializes it with a specific card number, and then asserts that the formatted string representation of the secret is equal to the expected masked card number.
    fn test_valid_card_number_masking() {
        let secret: Secret<String, CardNumberStrategy> =
            Secret::new("1234567890987654".to_string());
        assert_eq!("123456**********", format!("{secret:?}"));
    }

    #[test]
        /// This method tests the masking of an invalid card number by creating a Secret instance with a string value and asserting that the string representation of the Secret is masked as expected.
    fn test_invalid_card_number_masking() {
        let secret: Secret<String, CardNumberStrategy> = Secret::new("1234567890".to_string());
        assert_eq!("*** alloc::string::String ***", format!("{secret:?}"));
    }

    #[test]
        /// This function tests the strong secret masking functionality for a valid card number. It creates a CardNumber object from a string, retrieves a secret reference to the card number, and then asserts that the secret is masked as expected.
    fn test_valid_card_number_strong_secret_masking() {
        let card_number = CardNumber::from_str("3714 4963 5398 431").unwrap();
        let secret = &(*card_number);
        assert_eq!("371449*********", format!("{secret:?}"));
    }

    #[test]
        /// Deserialize a valid card number from a JSON string and then serialize it back to a string with sensitive information hidden.
    fn test_valid_card_number_deserialization() {
        let card_number = serde_json::from_str::<CardNumber>(r#""3714 4963 5398 431""#).unwrap();
        let secret = card_number.to_string();
        assert_eq!(r#""371449*********""#, format!("{secret:?}"));
    }

    #[test]
        /// Deserialize an invalid card number from JSON and assert that it results in an error with the correct message.
    fn test_invalid_card_number_deserialization() {
        let card_number = serde_json::from_str::<CardNumber>(r#""1234 5678""#);
        let error_msg = card_number.unwrap_err().to_string();
        assert_eq!(error_msg, "not a valid credit card number".to_string());
    }
}

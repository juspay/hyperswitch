use std::{fmt, ops::Deref, str::FromStr};

use masking::{PeekInterface, Strategy, StrongSecret, WithType};
use serde::{Deserialize, Deserializer, Serialize};
use thiserror::Error;

#[derive(Debug, Deserialize, Serialize, Error)]
#[error("not a valid credit card number")]
pub struct CCValError;

impl From<core::convert::Infallible> for CCValError {
    fn from(_: core::convert::Infallible) -> Self {
        Self
    }
}

/// Card number
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct CardNumber(StrongSecret<String, CardNumberStrategy>);

impl CardNumber {
    pub fn get_card_isin(self) -> String {
        self.0.peek().chars().take(6).collect::<String>()
    }
}

impl FromStr for CardNumber {
    type Err = CCValError;

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

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::from_str(&value)
    }
}

impl Deref for CardNumber {
    type Target = StrongSecret<String, CardNumberStrategy>;

    fn deref(&self) -> &StrongSecret<String, CardNumberStrategy> {
        &self.0
    }
}

impl<'de> Deserialize<'de> for CardNumber {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        Self::from_str(&s).map_err(serde::de::Error::custom)
    }
}

pub struct CardNumberStrategy;

impl<T> Strategy<T> for CardNumberStrategy
where
    T: AsRef<str>,
{
    fn fmt(val: &T, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let val_str: &str = val.as_ref();

        if val_str.len() < 15 || val_str.len() > 19 {
            return WithType::fmt(val, f);
        }

        write!(f, "{}{}", &val_str[..6], "*".repeat(val_str.len() - 6))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use masking::Secret;

    use super::*;

    #[test]
    fn valid_card_number() {
        let s = "371449635398431";
        assert_eq!(
            CardNumber::from_str(s).unwrap(),
            CardNumber(StrongSecret::from_str(s).unwrap())
        );
    }

    #[test]
    fn invalid_card_number() {
        let s = "371446431";
        assert_eq!(
            CardNumber::from_str(s).unwrap_err().to_string(),
            "not a valid credit card number".to_string()
        );
    }

    #[test]
    fn card_number_no_whitespace() {
        let s = "3714    4963  5398 431";
        assert_eq!(
            CardNumber::from_str(s).unwrap().to_string(),
            "371449*********"
        );
    }

    #[test]
    fn test_valid_card_number_masking() {
        let secret: Secret<String, CardNumberStrategy> =
            Secret::new("1234567890987654".to_string());
        assert_eq!("123456**********", format!("{secret:?}"));
    }

    #[test]
    fn test_invalid_card_number_masking() {
        let secret: Secret<String, CardNumberStrategy> = Secret::new("1234567890".to_string());
        assert_eq!("*** alloc::string::String ***", format!("{secret:?}"));
    }

    #[test]
    fn test_valid_card_number_strong_secret_masking() {
        let card_number = CardNumber::from_str("3714 4963 5398 431").unwrap();
        let secret = &(*card_number);
        assert_eq!("371449*********", format!("{secret:?}"));
    }

    #[test]
    fn test_valid_card_number_deserialization() {
        let card_number = serde_json::from_str::<CardNumber>(r#""3714 4963 5398 431""#).unwrap();
        let secret = card_number.to_string();
        assert_eq!(r#""371449*********""#, format!("{secret:?}"));
    }

    #[test]
    fn test_invalid_card_number_deserialization() {
        let card_number = serde_json::from_str::<CardNumber>(r#""1234 5678""#);
        let error_msg = card_number.unwrap_err().to_string();
        assert_eq!(error_msg, "not a valid credit card number".to_string());
    }
}

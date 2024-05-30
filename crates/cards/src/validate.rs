use std::{fmt, ops::Deref, str::FromStr};

use masking::{PeekInterface, Strategy, StrongSecret, WithType};
#[cfg(not(target_arch = "wasm32"))]
use router_env::{logger, which as router_env_which, Env};
use serde::{Deserialize, Deserializer, Serialize};
use thiserror::Error;

///
/// Minimum limit of a card number will not be less than 8 by ISO standards
///
pub const MIN_CARD_NUMBER_LENGTH: usize = 8;

///
/// Maximum limit of a card number will not exceed 19 by ISO standards
///
pub const MAX_CARD_NUMBER_LENGTH: usize = 19;

#[derive(Debug, Deserialize, Serialize, Error)]
#[error("{0}")]
pub struct CardNumberValidationErr(&'static str);

/// Card number
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct CardNumber(StrongSecret<String, CardNumberStrategy>);

impl CardNumber {
    pub fn get_card_isin(&self) -> String {
        self.0.peek().chars().take(6).collect::<String>()
    }

    pub fn get_extended_card_bin(&self) -> String {
        self.0.peek().chars().take(8).collect::<String>()
    }
    pub fn get_card_no(&self) -> String {
        self.0.peek().chars().collect::<String>()
    }
    pub fn get_last4(&self) -> String {
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
}

impl FromStr for CardNumber {
    type Err = CardNumberValidationErr;

    fn from_str(card_number: &str) -> Result<Self, Self::Err> {
        // Valid test cards for threedsecureio
        let valid_test_cards = vec![
            "4000100511112003",
            "6000100611111203",
            "3000100811111072",
            "9000100111111111",
        ];
        #[cfg(not(target_arch = "wasm32"))]
        let valid_test_cards = match router_env_which() {
            Env::Development | Env::Sandbox => valid_test_cards,
            Env::Production => vec![],
        };

        let card_number = card_number.split_whitespace().collect::<String>();

        let is_card_valid = sanitize_card_number(&card_number)?;

        if valid_test_cards.contains(&card_number.as_str()) || is_card_valid {
            Ok(Self(StrongSecret::new(card_number)))
        } else {
            Err(CardNumberValidationErr("card number invalid"))
        }
    }
}

pub fn sanitize_card_number(card_number: &str) -> Result<bool, CardNumberValidationErr> {
    let is_card_number_valid = Ok(card_number)
        .and_then(validate_card_number_chars)
        .and_then(validate_card_number_length)
        .map(|number| luhn(&number))?;

    Ok(is_card_number_valid)
}

///
/// # Panics
///
/// Never, as a single character will never be greater than 10, or `u8`
///
pub fn validate_card_number_chars(number: &str) -> Result<Vec<u8>, CardNumberValidationErr> {
    let data = number.chars().try_fold(
        Vec::with_capacity(MAX_CARD_NUMBER_LENGTH),
        |mut data, character| {
            data.push(
                #[allow(clippy::expect_used)]
                character
                    .to_digit(10)
                    .ok_or(CardNumberValidationErr(
                        "invalid character found in card number",
                    ))?
                    .try_into()
                    .expect("error while converting a single character to u8"), // safety, a single character will never be greater `u8`
            );
            Ok::<Vec<u8>, CardNumberValidationErr>(data)
        },
    )?;

    Ok(data)
}

pub fn validate_card_number_length(number: Vec<u8>) -> Result<Vec<u8>, CardNumberValidationErr> {
    if number.len() >= MIN_CARD_NUMBER_LENGTH && number.len() <= MAX_CARD_NUMBER_LENGTH {
        Ok(number)
    } else {
        Err(CardNumberValidationErr("invalid card number length"))
    }
}

#[allow(clippy::as_conversions)]
pub fn luhn(number: &[u8]) -> bool {
    number
        .iter()
        .rev()
        .enumerate()
        .map(|(idx, element)| {
            ((*element * 2) / 10 + (*element * 2) % 10) * ((idx as u8) % 2)
                + (*element) * (((idx + 1) as u8) % 2)
        })
        .sum::<u8>()
        % 10
        == 0
}

impl TryFrom<String> for CardNumber {
    type Error = CardNumberValidationErr;

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

pub enum CardNumberStrategy {}

impl<T> Strategy<T> for CardNumberStrategy
where
    T: AsRef<str>,
{
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
    fn valid_card_number() {
        let s = "371449635398431";
        assert_eq!(
            CardNumber::from_str(s).unwrap(),
            CardNumber(StrongSecret::from_str(s).unwrap())
        );
    }

    #[test]
    fn invalid_card_number_length() {
        let s = "371446";
        assert_eq!(
            CardNumber::from_str(s).unwrap_err().to_string(),
            "invalid card number length".to_string()
        );
    }

    #[test]
    fn card_number_with_non_digit_character() {
        let s = "371446431 A";
        assert_eq!(
            CardNumber::from_str(s).unwrap_err().to_string(),
            "invalid character found in card number".to_string()
        );
    }

    #[test]
    fn invalid_card_number() {
        let s = "371446431";
        assert_eq!(
            CardNumber::from_str(s).unwrap_err().to_string(),
            "card number invalid".to_string()
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
        assert_eq!(error_msg, "card number invalid".to_string());
    }
}

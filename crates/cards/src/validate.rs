use std::{collections::HashMap, fmt, ops::Deref, str::FromStr};

use common_utils::{errors::ValidationError, ext_traits::ConfigExt, types::LengthString};
use error_stack::report;
use masking::{PeekInterface, Secret, Strategy, StrongSecret, WithType};
use once_cell::sync::Lazy;
use regex::Regex;
#[cfg(not(target_arch = "wasm32"))]
use router_env::{logger, which as router_env_which, Env};
use serde::{Deserialize, Deserializer, Serialize};
use thiserror::Error;

/// Minimum limit of a card number will not be less than 8 by ISO standards
pub const MIN_CARD_NUMBER_LENGTH: usize = 8;

/// Maximum limit of a card number will not exceed 19 by ISO standards
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
    pub fn is_cobadged_card(&self) -> Result<bool, error_stack::Report<ValidationError>> {
        /// Regex to identify card networks
        static CARD_NETWORK_REGEX: Lazy<HashMap<&str, Result<Regex, regex::Error>>> = Lazy::new(
            || {
                let mut map = HashMap::new();
                map.insert("Mastercard", Regex::new(r"^(5[1-5][0-9]{14}|2(2(2[1-9]|[3-9][0-9])|[3-6][0-9][0-9]|7([0-1][0-9]|20))[0-9]{12})$"));
                map.insert("American Express", Regex::new(r"^3[47][0-9]{13}$"));
                map.insert("Visa", Regex::new(r"^4[0-9]{12}(?:[0-9]{3})?$"));
                map.insert("Discover", Regex::new(r"^65[4-9][0-9]{13}|64[4-9][0-9]{13}|6011[0-9]{12}|(622(?:12[6-9]|1[3-9][0-9]|[2-8][0-9][0-9]|9[01][0-9]|92[0-5])[0-9]{10})$"));
                map.insert(
        "Maestro",
        Regex::new(r"^(5018|5081|5044|504681|504993|5020|502260|5038|603845|603123|6304|6759|676[1-3]|6220|504834|504817|504645|504775|600206|627741)"),
    );
                map.insert(
        "RuPay",
        Regex::new(r"^(508227|508[5-9]|603741|60698[5-9]|60699|607[0-8]|6079[0-7]|60798[0-4]|60800[1-9]|6080[1-9]|608[1-4]|608500|6521[5-9]|652[2-9]|6530|6531[0-4]|817290|817368|817378|353800)"),
    );
                map.insert("Diners Club", Regex::new(r"^(36|38|30[0-5])"));
                map.insert(
                    "JCB",
                    Regex::new(r"^(3(?:088|096|112|158|337|5(?:2[89]|[3-8][0-9]))\d{12})$"),
                );
                map.insert("CarteBlanche", Regex::new(r"^389[0-9]{11}$"));
                map.insert("Sodex", Regex::new(r"^(637513)"));
                map.insert("BAJAJ", Regex::new(r"^(203040)"));
                map
            },
        );
        let mut no_of_supported_card_networks = 0;

        let card_number_str = self.get_card_no();
        for (_, regex) in CARD_NETWORK_REGEX.iter() {
            let card_regex = match regex.as_ref() {
                Ok(regex) => Ok(regex),
                Err(_) => Err(report!(ValidationError::InvalidValue {
                    message: "Invalid regex expression".into(),
                })),
            }?;

            if card_regex.is_match(&card_number_str) {
                no_of_supported_card_networks += 1;
                if no_of_supported_card_networks > 1 {
                    break;
                }
            }
        }
        Ok(no_of_supported_card_networks > 1)
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

/// # Panics
///
/// Never, as a single character will never be greater than 10, or `u8`
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

// #[derive(Debug, Deserialize, Serialize, Error)]
// #[error("{0}")]
// pub struct NameTypeValidationErr(&'static str);

#[derive(Clone, Default, Debug, Eq, PartialEq, Serialize)]
pub struct NameType(Secret<LengthString<256, 1>>);

impl TryFrom<String> for NameType {
    type Error = error_stack::Report<ValidationError>;
    fn try_from(card_holder_name: String) -> Result<Self, Self::Error> {
        for char in card_holder_name.chars() {
            validate_character_in_card_holder_name(char)?;
        }
        let valid_length_name =
            LengthString::<256, 1>::from(card_holder_name.into()).map_err(|_| {
                report!(ValidationError::InvalidValue {
                    message: "invalid length for name".to_string()
                })
            })?;
        Ok(Self(Secret::new(valid_length_name)))
    }
}

impl FromStr for NameType {
    type Err = error_stack::Report<ValidationError>;

    fn from_str(card_number: &str) -> Result<Self, Self::Err> {
        Self::try_from(card_number.to_string())
    }
}

impl From<NameType> for Secret<String> {
    fn from(card_holder_name: NameType) -> Self {
        Self::new(card_holder_name.peek().to_string())
    }
}

impl From<&NameType> for Secret<String> {
    fn from(card_holder_name: &NameType) -> Self {
        Self::new(card_holder_name.peek().to_string().to_owned())
    }
}

fn validate_character_in_card_holder_name(
    character: char,
) -> Result<(), error_stack::Report<ValidationError>> {
    if character.is_alphabetic()
        || character == ' '
        || character == '.'
        || character == '-'
        || character == '\''
        || character == '~'
        || character == '`'
    {
        Ok(())
    } else {
        Err(report!(ValidationError::InvalidValue {
            message: format!("invalid character found in card holder name: {}", character)
        }))
    }
}

impl<'de> Deserialize<'de> for NameType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let card_holder_name = String::deserialize(deserializer)?;
        card_holder_name
            .try_into()
            .map_err(serde::de::Error::custom)
    }
}

impl Deref for NameType {
    type Target = Secret<LengthString<256, 1>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ConfigExt for NameType {
    fn is_empty_after_trim(&self) -> bool {
        self.peek().trim().is_empty()
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
        let secret: Secret<String, CardNumberStrategy> = Secret::new("9123456789".to_string());
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

    // #[test]
    // fn test_card_holder_name() {

    //     let valid_name = LengthString::<256, 1>::try_from("Sakil O'Neil".to_string().into()).unwrap();
    //     // no panic
    //     let card_holder_name = NameType::try_from("Sakil O'Neil".to_string()).unwrap();

    //     // will panic on unwrap
    //     let invalid_card_holder_name = NameType::try_from("$@k!l M*$t@k".to_string());

    //     assert_eq!(*card_holder_name.peek(), valid_name);
    //     assert!(invalid_card_holder_name.is_err());

    //     let serialized = serde_json::to_string(&card_holder_name).unwrap();
    //     assert_eq!(&serialized, "\"Sakil O'Neil\"");

    //     let derialized = serde_json::from_str::<NameType>(&serialized).unwrap();
    //     assert_eq!(*derialized.peek(), valid_name);

    //     let invalid_deserialization = serde_json::from_str::<NameType>("$@k!l M*$t@k");
    //     assert!(invalid_deserialization.is_err());
    // }
}

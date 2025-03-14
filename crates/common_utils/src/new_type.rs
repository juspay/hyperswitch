//! Contains new types with restrictions
use masking::{ExposeInterface, PeekInterface, Secret};

use crate::{
    consts::MAX_ALLOWED_MERCHANT_NAME_LENGTH,
    pii::{Email, UpiVpaMaskingStrategy},
    transformers::ForeignFrom,
};

#[nutype::nutype(
    derive(Clone, Serialize, Deserialize, Debug),
    validate(len_char_min = 1, len_char_max = MAX_ALLOWED_MERCHANT_NAME_LENGTH)
)]
pub struct MerchantName(String);

impl masking::SerializableSecret for MerchantName {}

/// Function for masking alphanumeric characters in a string.
///
/// # Arguments
///     `val`
///         - holds reference to the string to be masked.
///     `unmasked_char_count`
///         - minimum character count to remain unmasked for identification
///         - this number is for keeping the characters unmasked from
///             both beginning (if feasible) and ending of the string.
///     `min_masked_char_count`
///         - this ensures the minimum number of characters to be masked
///
/// # Behaviour
///     - Returns the original string if its length is less than or equal to `unmasked_char_count`.
///     - If the string length allows, keeps `unmasked_char_count` characters unmasked at both start and end.
///     - Otherwise, keeps `unmasked_char_count` characters unmasked only at the end.
///     - Only alphanumeric characters are masked; other characters remain unchanged.
///
/// # Examples
///     Sort Code
///         (12-34-56, 2, 2) -> 12-**-56
///     Routing number
///         (026009593, 3, 3) -> 026***593
///     CNPJ
///         (12345678901, 4, 4) -> *******8901
///     CNPJ
///         (12345678901, 4, 3) -> 1234***8901
///     Pix key
///         (123e-a452-1243-1244-000, 4, 4) -> 123e-****-****-****-000
///     IBAN
///         (AL35202111090000000001234567, 5, 5) -> AL352******************34567
fn apply_mask(val: &str, unmasked_char_count: usize, min_masked_char_count: usize) -> String {
    let len = val.len();
    if len <= unmasked_char_count {
        return val.to_string();
    }

    let mask_start_index =
    // For showing only last `unmasked_char_count` characters
    if len < (unmasked_char_count * 2 + min_masked_char_count) {
        0
    // For showing first and last `unmasked_char_count` characters
    } else {
        unmasked_char_count
    };
    let mask_end_index = len - unmasked_char_count - 1;
    let range = mask_start_index..=mask_end_index;

    val.chars()
        .enumerate()
        .fold(String::new(), |mut acc, (index, ch)| {
            if ch.is_alphanumeric() && range.contains(&index) {
                acc.push('*');
            } else {
                acc.push(ch);
            }
            acc
        })
}

/// Masked sort code
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MaskedSortCode(Secret<String>);
impl From<String> for MaskedSortCode {
    fn from(src: String) -> Self {
        let masked_value = apply_mask(src.as_ref(), 2, 2);
        Self(Secret::from(masked_value))
    }
}
impl From<Secret<String>> for MaskedSortCode {
    fn from(secret: Secret<String>) -> Self {
        Self::from(secret.expose())
    }
}

/// Masked Routing number
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MaskedRoutingNumber(Secret<String>);
impl From<String> for MaskedRoutingNumber {
    fn from(src: String) -> Self {
        let masked_value = apply_mask(src.as_ref(), 3, 3);
        Self(Secret::from(masked_value))
    }
}
impl From<Secret<String>> for MaskedRoutingNumber {
    fn from(secret: Secret<String>) -> Self {
        Self::from(secret.expose())
    }
}

/// Masked bank account
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MaskedBankAccount(Secret<String>);
impl From<String> for MaskedBankAccount {
    fn from(src: String) -> Self {
        let masked_value = apply_mask(src.as_ref(), 4, 4);
        Self(Secret::from(masked_value))
    }
}
impl From<Secret<String>> for MaskedBankAccount {
    fn from(secret: Secret<String>) -> Self {
        Self::from(secret.expose())
    }
}

/// Masked IBAN
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MaskedIban(Secret<String>);
impl From<String> for MaskedIban {
    fn from(src: String) -> Self {
        let masked_value = apply_mask(src.as_ref(), 5, 5);
        Self(Secret::from(masked_value))
    }
}
impl From<Secret<String>> for MaskedIban {
    fn from(secret: Secret<String>) -> Self {
        Self::from(secret.expose())
    }
}

/// Masked IBAN
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MaskedBic(Secret<String>);
impl From<String> for MaskedBic {
    fn from(src: String) -> Self {
        let masked_value = apply_mask(src.as_ref(), 3, 2);
        Self(Secret::from(masked_value))
    }
}
impl From<Secret<String>> for MaskedBic {
    fn from(secret: Secret<String>) -> Self {
        Self::from(secret.expose())
    }
}

/// Masked UPI ID
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MaskedUpiVpaId(Secret<String>);
impl From<String> for MaskedUpiVpaId {
    fn from(src: String) -> Self {
        let unmasked_char_count = 2;
        let masked_value = if let Some((user_identifier, bank_or_psp)) = src.split_once('@') {
            let masked_user_identifier = user_identifier
                .to_string()
                .chars()
                .take(unmasked_char_count)
                .collect::<String>()
                + &"*".repeat(user_identifier.len() - unmasked_char_count);
            format!("{}@{}", masked_user_identifier, bank_or_psp)
        } else {
            let masked_value = apply_mask(src.as_ref(), unmasked_char_count, 8);
            masked_value
        };

        Self(Secret::from(masked_value))
    }
}
impl From<Secret<String, UpiVpaMaskingStrategy>> for MaskedUpiVpaId {
    fn from(secret: Secret<String, UpiVpaMaskingStrategy>) -> Self {
        Self::from(secret.expose())
    }
}

/// Masked Email
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MaskedEmail(Secret<String>);
impl From<String> for MaskedEmail {
    fn from(src: String) -> Self {
        let unmasked_char_count = 2;
        let masked_value = if let Some((user_identifier, domain)) = src.split_once('@') {
            let masked_user_identifier = user_identifier
                .to_string()
                .chars()
                .take(unmasked_char_count)
                .collect::<String>()
                + &"*".repeat(user_identifier.len() - unmasked_char_count);
            format!("{}@{}", masked_user_identifier, domain)
        } else {
            let masked_value = apply_mask(src.as_ref(), unmasked_char_count, 8);
            masked_value
        };
        Self(Secret::from(masked_value))
    }
}
impl From<Secret<String>> for MaskedEmail {
    fn from(secret: Secret<String>) -> Self {
        Self::from(secret.expose())
    }
}
impl ForeignFrom<Email> for MaskedEmail {
    fn foreign_from(email: Email) -> Self {
        let email_value: String = email.expose().peek().to_owned();
        Self::from(email_value)
    }
}

/// Masked Phone Number
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MaskedPhoneNumber(Secret<String>);
impl From<String> for MaskedPhoneNumber {
    fn from(src: String) -> Self {
        let unmasked_char_count = 2;
        let masked_value = if unmasked_char_count <= src.len() {
            let len = src.len();
            // mask every character except the last 2
            "*".repeat(len - unmasked_char_count).to_string()
                + src
                    .get(len.saturating_sub(unmasked_char_count)..)
                    .unwrap_or("")
        } else {
            src
        };
        Self(Secret::from(masked_value))
    }
}
impl From<Secret<String>> for MaskedPhoneNumber {
    fn from(secret: Secret<String>) -> Self {
        Self::from(secret.expose())
    }
}

#[cfg(test)]
mod apply_mask_fn_test {
    use masking::PeekInterface;

    use crate::new_type::{
        apply_mask, MaskedBankAccount, MaskedIban, MaskedRoutingNumber, MaskedSortCode,
        MaskedUpiVpaId,
    };
    #[test]
    fn test_masked_types() {
        let sort_code = MaskedSortCode::from("110011".to_string());
        let routing_number = MaskedRoutingNumber::from("056008849".to_string());
        let bank_account = MaskedBankAccount::from("12345678901234".to_string());
        let iban = MaskedIban::from("NL02ABNA0123456789".to_string());
        let upi_vpa = MaskedUpiVpaId::from("someusername@okhdfcbank".to_string());

        // Standard masked data tests
        assert_eq!(sort_code.0.peek().to_owned(), "11**11".to_string());
        assert_eq!(routing_number.0.peek().to_owned(), "056***849".to_string());
        assert_eq!(
            bank_account.0.peek().to_owned(),
            "1234******1234".to_string()
        );
        assert_eq!(iban.0.peek().to_owned(), "NL02A********56789".to_string());
        assert_eq!(
            upi_vpa.0.peek().to_owned(),
            "so**********@okhdfcbank".to_string()
        );
    }

    #[test]
    fn test_apply_mask_fn() {
        let value = "12345678901".to_string();

        // Generic masked tests
        assert_eq!(apply_mask(&value, 2, 2), "12*******01".to_string());
        assert_eq!(apply_mask(&value, 3, 2), "123*****901".to_string());
        assert_eq!(apply_mask(&value, 3, 3), "123*****901".to_string());
        assert_eq!(apply_mask(&value, 4, 3), "1234***8901".to_string());
        assert_eq!(apply_mask(&value, 4, 4), "*******8901".to_string());
        assert_eq!(apply_mask(&value, 5, 4), "******78901".to_string());
        assert_eq!(apply_mask(&value, 5, 5), "******78901".to_string());
        assert_eq!(apply_mask(&value, 6, 5), "*****678901".to_string());
        assert_eq!(apply_mask(&value, 6, 6), "*****678901".to_string());
        assert_eq!(apply_mask(&value, 7, 6), "****5678901".to_string());
        assert_eq!(apply_mask(&value, 7, 7), "****5678901".to_string());
        assert_eq!(apply_mask(&value, 8, 7), "***45678901".to_string());
        assert_eq!(apply_mask(&value, 8, 8), "***45678901".to_string());
    }
}

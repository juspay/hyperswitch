//!
//! Secret truncated strings
//!
//! Using type aliases for truncating and masking different types of string values.

use alloc::string::{String, ToString};

use super::SerializableSecret;

/// Function for masking alphanumeric characters in a string
/// examples
///  Sort Code
///   (12-34-56, 2, 2) -> 12-**-56
///  Routing number
///   (026009593, 3, 3) -> 026***593
///  CNPJ
///   (12345678901, 4, 4) -> 1234***8901
///  Pix key
///   (123e-a452-1243-1244-000, 4, 4) -> 123e-****-****-****-000
///  IBAN
///   (AL35202111090000000001234567, 5, 5) -> AL352******************34567
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

    val.chars()
        .enumerate()
        .fold(String::new(), |mut acc, (index, ch)| {
            if ch.is_alphanumeric() && (mask_start_index..=mask_end_index).contains(&index) {
                acc.push('*');
            } else {
                acc.push(ch);
            }
            acc
        })
}

/// Masked sort code

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MaskedSortCode(pub String);

impl SerializableSecret for MaskedSortCode {}
impl From<String> for MaskedSortCode {
    fn from(src: String) -> Self {
        let masked_value = apply_mask(src.as_ref(), 2, 2);
        Self(masked_value)
    }
}

/// Masked Routing number

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MaskedRoutingNumber(pub String);

impl SerializableSecret for MaskedRoutingNumber {}
impl From<String> for MaskedRoutingNumber {
    fn from(src: String) -> Self {
        let masked_value = apply_mask(src.as_ref(), 3, 3);
        Self(masked_value)
    }
}

/// Masked bank account

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MaskedBankAccount(pub String);

impl SerializableSecret for MaskedBankAccount {}
impl From<String> for MaskedBankAccount {
    fn from(src: String) -> Self {
        let masked_value = apply_mask(src.as_ref(), 4, 4);
        Self(masked_value)
    }
}

/// Masked IBAN

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MaskedIban(pub String);

impl SerializableSecret for MaskedIban {}
impl From<String> for MaskedIban {
    fn from(src: String) -> Self {
        let masked_value = apply_mask(src.as_ref(), 5, 5);
        Self(masked_value)
    }
}

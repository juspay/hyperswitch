use std::str::FromStr;

use common_utils::types::MinorUnit;

pub struct Unpacker;

pub fn string_to_minor_unit_cardbin(card_bin: &str) -> Option<MinorUnit> {
    match i64::from_str(card_bin.trim()) {
        Ok(num) if num >= 0 => Some(MinorUnit::new(num)),
        _ => None,
    }
}

//! Types that can be used in other crates
use error_stack::{IntoReport, ResultExt};
use serde::{de::Visitor, Deserialize, Deserializer};

use crate::{
    consts,
    errors::{CustomResult, PercentageError},
};

/// Represents Percentage Value between 0 and 100 both inclusive
#[derive(Clone, Default, Debug, PartialEq, serde::Serialize)]
pub struct Percentage<const PRECISION: u8> {
    // this value will range from 0 to 100, decimal length defined by precision macro
    /// Percentage value ranging between 0 and 100
    percentage: f32,
}

/// Returns an error message for an invalid percentage value.
/// 
/// # Arguments
///
/// * `precision` - The precision of the percentage value
///
/// # Return
///
/// A string containing the error message for an invalid percentage value
///
fn get_invalid_percentage_error_message(precision: u8) -> String {
    format!(
        "value should be a float between 0 to 100 and precise to only upto {} decimal digits",
        precision
    )
}

impl<const PRECISION: u8> Percentage<PRECISION> {
    /// construct percentage using a string representation of float value
    pub fn from_string(value: String) -> CustomResult<Self, PercentageError> {
        if Self::is_valid_string_value(&value)? {
            Ok(Self {
                percentage: value
                    .parse()
                    .into_report()
                    .change_context(PercentageError::InvalidPercentageValue)?,
            })
        } else {
            Err(PercentageError::InvalidPercentageValue.into())
                .attach_printable(get_invalid_percentage_error_message(PRECISION))
        }
    }
    /// function to get percentage value
    pub fn get_percentage(&self) -> f32 {
        self.percentage
    }

    /// apply the percentage to amount and ceil the result
    #[allow(clippy::as_conversions)]
        /// Applies the percentage value of the struct instance to the given amount and returns the result rounded up to the nearest integer.
    /// If the given amount is greater than i64::MAX/10000, it returns a PercentageError::UnableToApplyPercentage with a printable message attached.
    pub fn apply_and_ceil_result(&self, amount: i64) -> CustomResult<i64, PercentageError> {
        let max_amount = i64::MAX / 10000;
        if amount > max_amount {
            // value gets rounded off after i64::MAX/10000
            Err(PercentageError::UnableToApplyPercentage {
                percentage: self.percentage,
                amount,
            }
            .into())
            .attach_printable(format!(
                "Cannot calculate percentage for amount greater than {}",
                max_amount
            ))
        } else {
            let percentage_f64 = f64::from(self.percentage);
            let result = (amount as f64 * (percentage_f64 / 100.0)).ceil() as i64;
            Ok(result)
        }
    }
        /// Takes a string value and checks if it is a valid float string. If it is, then it checks if the float value falls within a valid range and if the precision length of the string is valid. Returns a CustomResult containing a boolean value indicating whether the string value is valid or a PercentageError if any of the checks fail.
    fn is_valid_string_value(value: &str) -> CustomResult<bool, PercentageError> {
        let float_value = Self::is_valid_float_string(value)?;
        Ok(Self::is_valid_range(float_value) && Self::is_valid_precision_length(value))
    }
        /// Parses the given string value as a floating-point number and returns a `CustomResult`
    /// containing the parsed floating-point number or a `PercentageError` if the value
    /// is invalid.
    fn is_valid_float_string(value: &str) -> CustomResult<f32, PercentageError> {
        value
            .parse()
            .into_report()
            .change_context(PercentageError::InvalidPercentageValue)
    }
        /// Checks if the given value falls within the range of 0.0 to 100.0 (inclusive).
    /// 
    /// # Arguments
    /// 
    /// * `value` - A 32-bit floating point number to be checked for validity.
    /// 
    /// # Returns
    /// 
    /// A boolean value indicating whether the input value falls within the specified range.
    fn is_valid_range(value: f32) -> bool {
        (0.0..=100.0).contains(&value)
    }
        /// Checks if the precision length of the decimal part in the given string value is valid based on a predefined precision constant.
    fn is_valid_precision_length(value: &str) -> bool {
        if value.contains('.') {
            // if string has '.' then take the decimal part and verify precision length
            match value.split('.').last() {
                Some(decimal_part) => {
                    decimal_part.trim_end_matches('0').len() <= <u8 as Into<usize>>::into(PRECISION)
                }
                // will never be None
                None => false,
            }
        } else {
            // if there is no '.' then it is a whole number with no decimal part. So return true
            true
        }
    }
}

// custom serde deserialization function
struct PercentageVisitor<const PRECISION: u8> {}
impl<'de, const PRECISION: u8> Visitor<'de> for PercentageVisitor<PRECISION> {
    type Value = Percentage<PRECISION>;

        /// This method is used to write a string to the given formatter, indicating that it is expecting a Percentage object.
    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Percentage object")
    }
        /// Visits the map and extracts the percentage value, returning the result as a Percentage type.
    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        let mut percentage_value = None;
        while let Some(key) = map.next_key::<String>()? {
            if key.eq("percentage") {
                if percentage_value.is_some() {
                    return Err(serde::de::Error::duplicate_field("percentage"));
                }
                percentage_value = Some(map.next_value::<serde_json::Value>()?);
            } else {
                // Ignore unknown fields
                let _: serde::de::IgnoredAny = map.next_value()?;
            }
        }
        if let Some(value) = percentage_value {
            let string_value = value.to_string();
            Ok(Percentage::from_string(string_value.clone()).map_err(|_| {
                serde::de::Error::invalid_value(
                    serde::de::Unexpected::Other(&format!("percentage value {}", string_value)),
                    &&*get_invalid_percentage_error_message(PRECISION),
                )
            })?)
        } else {
            Err(serde::de::Error::missing_field("percentage"))
        }
    }
}

impl<'de, const PRECISION: u8> Deserialize<'de> for Percentage<PRECISION> {
        /// Deserialize the data using the provided Deserializer and return the result.
    fn deserialize<D>(data: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        data.deserialize_map(PercentageVisitor::<PRECISION> {})
    }
}

/// represents surcharge type and value
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case", tag = "type", content = "value")]
pub enum Surcharge {
    /// Fixed Surcharge value
    Fixed(i64),
    /// Surcharge percentage
    Rate(Percentage<{ consts::SURCHARGE_PERCENTAGE_PRECISION_LENGTH }>),
}

use std::ops::Add;

use common_utils::errors::{ApiModelsError, CustomResult};
use error_stack::{report, ResultExt};
use serde::{de::Visitor, Deserialize, Deserializer};
use utoipa::ToSchema;

#[derive(Clone, Default, Debug, PartialEq, serde::Serialize, ToSchema)]
pub struct Percentage<const PRECISION: usize> {
    // this value will range from 0 to 100, decimal length defined by precision macro
    /// Percentage value ranging between 0 and 100
    #[schema(example = 2.5)]
    percentage: f32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Private;

fn get_invalid_percentage_error_message(precision: usize) -> String {
    format!(
        "value should be between 0 to 100 and precise to only upto {} decimal digits",
        precision
    )
}

impl<const PRECISION: usize> Percentage<PRECISION> {
    pub fn from_float(value: f32) -> CustomResult<Self, ApiModelsError> {
        if Self::is_valid_value(value)? {
            Ok(Self { percentage: value })
        } else {
            Err(ApiModelsError::InvalidPercentageValue.into())
                .attach_printable(get_invalid_percentage_error_message(PRECISION))
        }
    }
    pub fn get_percentage(&self) -> f32 {
        self.percentage
    }
    fn is_valid_range(value: f32) -> bool {
        (0.0..=100.0).contains(&value)
    }
    fn is_valid_value(value: f32) -> CustomResult<bool, ApiModelsError> {
        Ok(Self::is_valid_range(value) && Self::is_valid_precision_length(value)?)
    }
    fn is_valid_precision_length(value: f32) -> CustomResult<bool, ApiModelsError> {
        // convert to string
        let mut string_representation = value.to_string();
        if string_representation.find('.').is_none() {
            // if '.' is not found in string representation, it is a whole number, so we append a '.' at the end
            string_representation = string_representation.add(".");
        }
        // take the decimal part
        let decimal_part = string_representation
            .split('.')
            .last()
            .ok_or(report!(ApiModelsError::InvalidPercentageValue))
            .attach_printable("Error while verifying percentage value")?;
        if PRECISION >= decimal_part.len() {
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

struct PercentageVisitor<const PRECISION: usize> {}
impl<'de, const PRECISION: usize> Visitor<'de> for PercentageVisitor<PRECISION> {
    type Value = Percentage<PRECISION>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Percentage object")
    }
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
                percentage_value = Some(map.next_value::<f32>()?);
            } else {
                // Ignore unknown fields
                let _: serde::de::IgnoredAny = map.next_value()?;
            }
        }
        if let Some(value) = percentage_value {
            let str_value = value.to_string();
            Ok(Percentage::from_float(value).map_err(|_| {
                serde::de::Error::invalid_value(
                    serde::de::Unexpected::Other(&format!("percentage value `{}`", str_value)),
                    &&*get_invalid_percentage_error_message(PRECISION),
                )
            })?)
        } else {
            Err(serde::de::Error::missing_field("percentage"))
        }
    }
}

impl<'de, const PRECISION: usize> Deserialize<'de> for Percentage<PRECISION> {
    fn deserialize<D>(data: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        data.deserialize_map(PercentageVisitor::<PRECISION> {})
    }
}

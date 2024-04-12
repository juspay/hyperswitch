//! Types that can be used in other crates
use std::{fmt::Display, str::FromStr};

use diesel::{
    backend::Backend,
    deserialize::FromSql,
    serialize::{Output, ToSql},
    sql_types::Jsonb,
    AsExpression, FromSqlRow,
};
use error_stack::{report, ResultExt};
use semver::Version;
use serde::{de::Visitor, Deserialize, Deserializer};

use crate::{
    consts,
    errors::{CustomResult, ParsingError, PercentageError},
};
/// Represents Percentage Value between 0 and 100 both inclusive
#[derive(Clone, Default, Debug, PartialEq, serde::Serialize)]
pub struct Percentage<const PRECISION: u8> {
    // this value will range from 0 to 100, decimal length defined by precision macro
    /// Percentage value ranging between 0 and 100
    percentage: f32,
}

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
                    .parse::<f32>()
                    .change_context(PercentageError::InvalidPercentageValue)?,
            })
        } else {
            Err(report!(PercentageError::InvalidPercentageValue))
                .attach_printable(get_invalid_percentage_error_message(PRECISION))
        }
    }
    /// function to get percentage value
    pub fn get_percentage(&self) -> f32 {
        self.percentage
    }

    /// apply the percentage to amount and ceil the result
    #[allow(clippy::as_conversions)]
    pub fn apply_and_ceil_result(&self, amount: i64) -> CustomResult<i64, PercentageError> {
        let max_amount = i64::MAX / 10000;
        if amount > max_amount {
            // value gets rounded off after i64::MAX/10000
            Err(report!(PercentageError::UnableToApplyPercentage {
                percentage: self.percentage,
                amount,
            }))
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
    fn is_valid_string_value(value: &str) -> CustomResult<bool, PercentageError> {
        let float_value = Self::is_valid_float_string(value)?;
        Ok(Self::is_valid_range(float_value) && Self::is_valid_precision_length(value))
    }
    fn is_valid_float_string(value: &str) -> CustomResult<f32, PercentageError> {
        value
            .parse::<f32>()
            .change_context(PercentageError::InvalidPercentageValue)
    }
    fn is_valid_range(value: f32) -> bool {
        (0.0..=100.0).contains(&value)
    }
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

/// This struct lets us represent a semantic version type
#[derive(Debug, Clone, PartialEq, Eq, FromSqlRow, AsExpression)]
#[diesel(sql_type = Jsonb)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct SemanticVersion(#[serde(with = "Version")] Version);

impl SemanticVersion {
    /// returns major version number
    pub fn get_major(&self) -> u64 {
        self.0.major
    }
}

impl Display for SemanticVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for SemanticVersion {
    type Err = error_stack::Report<ParsingError>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(Version::from_str(s).change_context(
            ParsingError::StructParseFailure("SemanticVersion"),
        )?))
    }
}

impl<DB: Backend> FromSql<Jsonb, DB> for SemanticVersion
where
    serde_json::Value: FromSql<Jsonb, DB>,
{
    fn from_sql(bytes: DB::RawValue<'_>) -> diesel::deserialize::Result<Self> {
        let value = <serde_json::Value as FromSql<Jsonb, DB>>::from_sql(bytes)?;
        Ok(serde_json::from_value(value)?)
    }
}

impl ToSql<Jsonb, diesel::pg::Pg> for SemanticVersion
where
    serde_json::Value: ToSql<Jsonb, diesel::pg::Pg>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, diesel::pg::Pg>) -> diesel::serialize::Result {
        let value = serde_json::to_value(self)?;

        // the function `reborrow` only works in case of `Pg` backend. But, in case of other backends
        // please refer to the diesel migration blog:
        // https://github.com/Diesel-rs/Diesel/blob/master/guide_drafts/migration_guide.md#changed-tosql-implementations
        <serde_json::Value as ToSql<Jsonb, diesel::pg::Pg>>::to_sql(&value, &mut out.reborrow())
    }
}

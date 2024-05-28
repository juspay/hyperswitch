//! Types that can be used in other crates
use std::{
    fmt::Display,
    ops::{Add, Sub},
    primitive::i64,
    str::FromStr,
};

use common_enums::enums;
use diesel::{
    backend::Backend,
    deserialize,
    deserialize::FromSql,
    serialize::{Output, ToSql},
    sql_types,
    sql_types::Jsonb,
    AsExpression, FromSqlRow, Queryable,
};
use error_stack::{report, ResultExt};
use rust_decimal::{
    prelude::{FromPrimitive, ToPrimitive},
    Decimal,
};
use semver::Version;
use serde::{de::Visitor, Deserialize, Deserializer};
use utoipa::ToSchema;

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
    pub fn apply_and_ceil_result(
        &self,
        amount: MinorUnit,
    ) -> CustomResult<MinorUnit, PercentageError> {
        let max_amount = i64::MAX / 10000;
        let amount = amount.0;
        if amount > max_amount {
            // value gets rounded off after i64::MAX/10000
            Err(report!(PercentageError::UnableToApplyPercentage {
                percentage: self.percentage,
                amount: MinorUnit::new(amount),
            }))
            .attach_printable(format!(
                "Cannot calculate percentage for amount greater than {}",
                max_amount
            ))
        } else {
            let percentage_f64 = f64::from(self.percentage);
            let result = (amount as f64 * (percentage_f64 / 100.0)).ceil() as i64;
            Ok(MinorUnit::new(result))
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
    Fixed(MinorUnit),
    /// Surcharge percentage
    Rate(Percentage<{ consts::SURCHARGE_PERCENTAGE_PRECISION_LENGTH }>),
}

/// This struct lets us represent a semantic version type
#[derive(Debug, Clone, PartialEq, Eq, FromSqlRow, AsExpression, Ord, PartialOrd)]
#[diesel(sql_type = Jsonb)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct SemanticVersion(#[serde(with = "Version")] Version);

impl SemanticVersion {
    /// returns major version number
    pub fn get_major(&self) -> u64 {
        self.0.major
    }
    /// Constructs new SemanticVersion instance
    pub fn new(major: u64, minor: u64, patch: u64) -> Self {
        Self(Version::new(major, minor, patch))
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
    fn from_sql(bytes: DB::RawValue<'_>) -> deserialize::Result<Self> {
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

/// Amount convertor trait for connector
pub trait AmountConvertor: Send {
    /// Output type for the connector
    type Output;
    /// helps in conversion of connector required amount type
    fn convert(
        &self,
        i: MinorUnit,
        currency: enums::Currency,
    ) -> Result<Self::Output, error_stack::Report<ParsingError>>;

    /// helps in converting back connector required amount type to core minor unit
    fn convert_back(
        &self,
        i: Self::Output,
        currency: enums::Currency,
    ) -> Result<MinorUnit, error_stack::Report<ParsingError>>;
}

/// Connector required amount type
#[derive(Default, Debug, serde::Deserialize, serde::Serialize, Clone, Copy, PartialEq)]
pub struct StringMinorUnitForConnector;

impl AmountConvertor for StringMinorUnitForConnector {
    type Output = StringMinorUnit;
    fn convert(
        &self,
        i: MinorUnit,
        _currency: enums::Currency,
    ) -> Result<Self::Output, error_stack::Report<ParsingError>> {
        i.to_minor_unit_as_string()
    }

    fn convert_back(
        &self,
        i: Self::Output,
        _currency: enums::Currency,
    ) -> Result<MinorUnit, error_stack::Report<ParsingError>> {
        i.to_minor_unit_as_i64()
    }
}

/// Connector required amount type
#[derive(Default, Debug, serde::Deserialize, serde::Serialize, Clone, Copy, PartialEq)]
pub struct StringMajorUnitForConnector;

impl AmountConvertor for StringMajorUnitForConnector {
    type Output = StringMajorUnit;
    fn convert(
        &self,
        i: MinorUnit,
        currency: enums::Currency,
    ) -> Result<Self::Output, error_stack::Report<ParsingError>> {
        i.to_major_unit_as_string(currency)
    }

    fn convert_back(
        &self,
        i: StringMajorUnit,
        currency: enums::Currency,
    ) -> Result<MinorUnit, error_stack::Report<ParsingError>> {
        i.to_minor_unit_as_i64(currency)
    }
}

/// Connector required amount type
#[derive(Default, Debug, serde::Deserialize, serde::Serialize, Clone, Copy, PartialEq)]
pub struct FloatMajorUnitForConnector;

impl AmountConvertor for FloatMajorUnitForConnector {
    type Output = FloatMajorUnit;
    fn convert(
        &self,
        i: MinorUnit,
        currency: enums::Currency,
    ) -> Result<Self::Output, error_stack::Report<ParsingError>> {
        i.to_major_unit_as_f64(currency)
    }
    fn convert_back(
        &self,
        i: FloatMajorUnit,
        currency: enums::Currency,
    ) -> Result<MinorUnit, error_stack::Report<ParsingError>> {
        i.to_minor_unit_as_i64(currency)
    }
}

/// This Unit struct represents MinorUnit in which core amount works
#[derive(
    Default,
    Debug,
    serde::Deserialize,
    AsExpression,
    serde::Serialize,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    ToSchema,
    PartialOrd,
)]
#[diesel(sql_type = sql_types::BigInt)]
pub struct MinorUnit(i64);

impl MinorUnit {
    /// gets amount as i64 value will be removed in future
    pub fn get_amount_as_i64(&self) -> i64 {
        self.0
    }

    /// forms a new minor unit from amount
    pub fn new(value: i64) -> Self {
        Self(value)
    }

    /// Convert the amount to its major denomination based on Currency and return String
    pub fn to_major_unit_as_string(
        &self,
        currency: enums::Currency,
    ) -> Result<StringMajorUnit, error_stack::Report<ParsingError>> {
        let amount_f64 = self.to_major_unit_as_f64(currency)?;
        let amount_string = if currency.is_three_decimal_currency() {
            format!("{:.3}", amount_f64.0)
        } else {
            format!("{:.2}", amount_f64.0)
        };
        Ok(StringMajorUnit::new(amount_string))
    }

    /// Convert the amount to its major denomination based on Currency and return f64
    pub fn to_major_unit_as_f64(
        &self,
        currency: enums::Currency,
    ) -> Result<FloatMajorUnit, error_stack::Report<ParsingError>> {
        let amount_decimal =
            Decimal::from_i64(self.0).ok_or(ParsingError::I64ToDecimalConversionFailure)?;
        let amount_f64 = amount_decimal
            .to_f64()
            .ok_or(ParsingError::FloatToDecimalConversionFailure)?;
        let amount = if currency.is_zero_decimal_currency() {
            amount_f64
        } else if currency.is_three_decimal_currency() {
            amount_f64 / 1000.00
        } else {
            amount_f64 / 100.00
        };
        Ok(FloatMajorUnit::new(amount))
    }

    ///Convert the higher decimal amount to its major absolute units
    pub fn to_minor_unit_as_string(
        &self,
    ) -> Result<StringMinorUnit, error_stack::Report<ParsingError>> {
        let amount_decimal =
            Decimal::from_i64(self.0).ok_or(ParsingError::I64ToDecimalConversionFailure)?;
        Ok(StringMinorUnit::new(amount_decimal.to_string()))
    }

    /// Convert the amount to its major denomination based on Currency and check for zero decimal currency and return String
    /// Paypal Connector accepts Zero and Two decimal currency but not three decimal and it should be updated as required for 3 decimal currencies.
    /// Paypal Ref - https://developer.paypal.com/docs/reports/reference/paypal-supported-currencies/
    pub fn to_major_unit_as_string_with_zero_decimal_check(
        &self,
        currency: enums::Currency,
    ) -> Result<StringMajorUnit, error_stack::Report<ParsingError>> {
        let amount_f64 = self.to_major_unit_as_f64(currency)?;
        if currency.is_zero_decimal_currency() {
            Ok(StringMajorUnit::new(amount_f64.0.to_string()))
        } else {
            let amount_string = format!("{:.2}", amount_f64.0);
            Ok(StringMajorUnit::new(amount_string))
        }
    }
}

impl Display for MinorUnit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<DB> FromSql<sql_types::BigInt, DB> for MinorUnit
where
    DB: Backend,
    i64: FromSql<sql_types::BigInt, DB>,
{
    fn from_sql(value: DB::RawValue<'_>) -> deserialize::Result<Self> {
        let val = i64::from_sql(value)?;
        Ok(Self(val))
    }
}

impl<DB> ToSql<sql_types::BigInt, DB> for MinorUnit
where
    DB: Backend,
    i64: ToSql<sql_types::BigInt, DB>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> diesel::serialize::Result {
        self.0.to_sql(out)
    }
}

impl<DB> Queryable<sql_types::BigInt, DB> for MinorUnit
where
    DB: Backend,
    Self: FromSql<sql_types::BigInt, DB>,
{
    type Row = Self;

    fn build(row: Self::Row) -> deserialize::Result<Self> {
        Ok(row)
    }
}

impl Add for MinorUnit {
    type Output = Self;
    fn add(self, a2: Self) -> Self {
        Self(self.0 + a2.0)
    }
}

impl Sub for MinorUnit {
    type Output = Self;
    fn sub(self, a2: Self) -> Self {
        Self(self.0 - a2.0)
    }
}

/// Connector specific types to send

#[derive(Default, Debug, serde::Deserialize, serde::Serialize, Clone, PartialEq)]
pub struct StringMinorUnit(String);

impl StringMinorUnit {
    /// forms a new minor unit in string from amount
    pub fn new(value: String) -> Self {
        Self(value)
    }

    /// converts to i64 from minor unit string value
    pub fn to_minor_unit_as_i64(&self) -> Result<MinorUnit, error_stack::Report<ParsingError>> {
        let amount_string = &self.0;
        let amount_decimal = Decimal::from_str(amount_string)
            .map_err(|_| ParsingError::StringToDecimalConversionFailure)?;
        let amount_decimal_scaled = amount_decimal.round();
        let amount_i64 = amount_decimal_scaled
            .to_i64()
            .ok_or(ParsingError::DecimalToI64ConversionFailure)?;
        Ok(MinorUnit::new(amount_i64))
    }
}

/// Connector specific types to send
#[derive(Default, Debug, serde::Deserialize, serde::Serialize, Clone, Copy, PartialEq)]
pub struct FloatMajorUnit(f64);

impl FloatMajorUnit {
    /// forms a new major unit from amount
    pub fn new(value: f64) -> Self {
        Self(value)
    }

    /// converts to minor unit as i64 from FloatMajorUnit
    pub fn to_minor_unit_as_i64(
        &self,
        currency: enums::Currency,
    ) -> Result<MinorUnit, error_stack::Report<ParsingError>> {
        let amount_f64 = self.0;
        let amount = if currency.is_zero_decimal_currency() {
            amount_f64
        } else if currency.is_three_decimal_currency() {
            amount_f64 * 1000.00
        } else {
            amount_f64 * 100.00
        };

        let amount_decimal =
            Decimal::from_f64(amount).ok_or(ParsingError::FloatToDecimalConversionFailure)?;
        let amount_decimal_scaled = amount_decimal.round();
        let amount_i64 = amount_decimal_scaled
            .to_i64()
            .ok_or(ParsingError::DecimalToI64ConversionFailure)?;
        Ok(MinorUnit::new(amount_i64))
    }
}

/// Connector specific types to send
#[derive(Default, Debug, serde::Deserialize, serde::Serialize, Clone, PartialEq, Eq)]
pub struct StringMajorUnit(String);

impl StringMajorUnit {
    /// forms a new major unit from amount
    pub fn new(value: String) -> Self {
        Self(value)
    }

    /// Converts to minor unit as i64 from StringMajorUnit
    pub fn to_minor_unit_as_i64(
        &self,
        currency: enums::Currency,
    ) -> Result<MinorUnit, error_stack::Report<ParsingError>> {
        let amount_f64 = self
            .0
            .parse::<f64>()
            .change_context(ParsingError::StringToFloatConversionFailure)?;
        let amount = if currency.is_zero_decimal_currency() {
            amount_f64
        } else if currency.is_three_decimal_currency() {
            amount_f64 * 1000.00
        } else {
            amount_f64 * 100.00
        };

        let amount_decimal =
            Decimal::from_f64(amount).ok_or(ParsingError::FloatToDecimalConversionFailure)?;
        let amount_decimal_scaled = amount_decimal.round();
        let amount_i64 = amount_decimal_scaled
            .to_i64()
            .ok_or(ParsingError::DecimalToI64ConversionFailure)?;
        Ok(MinorUnit::new(amount_i64))
    }
}

#[cfg(test)]
mod amount_conversion_tests {
    use super::*;
    #[test]
    fn amount_conversion_to_float_major_unit() {
        let request_amount = MinorUnit::new(999999999);
        let two_decimal_currency = enums::Currency::USD;
        let three_decimal_currency = enums::Currency::BHD;
        let required_conversion = FloatMajorUnitForConnector;

        // Two decimal currency conversions
        let converted_amount = required_conversion
            .convert(request_amount, two_decimal_currency)
            .unwrap();
        assert_eq!(converted_amount.0, 9999999.99);
        let converted_back_amount = required_conversion
            .convert_back(converted_amount, two_decimal_currency)
            .unwrap();
        assert_eq!(converted_back_amount, request_amount);

        // Three decimal currency conversions
        let converted_amount = required_conversion
            .convert(request_amount, three_decimal_currency)
            .unwrap();
        assert_eq!(converted_amount.0, 999999.999);
        let converted_back_amount = required_conversion
            .convert_back(converted_amount, three_decimal_currency)
            .unwrap();
        assert_eq!(converted_back_amount, request_amount);
    }

    #[test]
    fn amount_conversion_to_string_major_unit() {
        let request_amount = MinorUnit::new(999999999);
        let two_decimal_currency = enums::Currency::USD;
        let three_decimal_currency = enums::Currency::BHD;
        let required_conversion = StringMajorUnitForConnector;

        // Two decimal currency conversions
        let converted_amount_two_decimal_currency = required_conversion
            .convert(request_amount, two_decimal_currency)
            .unwrap();
        assert_eq!(
            converted_amount_two_decimal_currency.0,
            "9999999.99".to_string()
        );
        let converted_back_amount = required_conversion
            .convert_back(converted_amount_two_decimal_currency, two_decimal_currency)
            .unwrap();
        assert_eq!(converted_back_amount, request_amount);

        // Three decimal currency conversions
        let converted_amount_three_decimal_currency = required_conversion
            .convert(request_amount, three_decimal_currency)
            .unwrap();
        assert_eq!(
            converted_amount_three_decimal_currency.0,
            "999999.999".to_string()
        );
        let converted_back_amount = required_conversion
            .convert_back(
                converted_amount_three_decimal_currency,
                three_decimal_currency,
            )
            .unwrap();
        assert_eq!(converted_back_amount, request_amount);
    }

    #[test]
    fn amount_conversion_to_string_minor_unit() {
        let request_amount = MinorUnit::new(999999999);
        let currency = enums::Currency::USD;
        let required_conversion = StringMinorUnitForConnector;
        let converted_amount = required_conversion
            .convert(request_amount, currency)
            .unwrap();
        assert_eq!(converted_amount.0, "999999999".to_string());
        let converted_back_amount = required_conversion
            .convert_back(converted_amount, currency)
            .unwrap();
        assert_eq!(converted_back_amount, request_amount);
    }
}

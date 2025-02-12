//! Types that can be used in other crates
pub mod keymanager;

/// Enum for Authentication Level
pub mod authentication;
/// Enum for Theme Lineage
pub mod theme;

/// types that are wrappers around primitive types
pub mod primitive_wrappers;

use std::{
    borrow::Cow,
    fmt::Display,
    iter::Sum,
    ops::{Add, Mul, Sub},
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
pub use primitive_wrappers::bool_wrappers::{
    AlwaysRequestExtendedAuthorization, ExtendedAuthorizationAppliedBool,
    RequestExtendedAuthorizationBool,
};
use rust_decimal::{
    prelude::{FromPrimitive, ToPrimitive},
    Decimal,
};
use semver::Version;
use serde::{de::Visitor, Deserialize, Deserializer, Serialize};
use thiserror::Error;
use time::PrimitiveDateTime;
use utoipa::ToSchema;

use crate::{
    consts::{
        self, MAX_DESCRIPTION_LENGTH, MAX_STATEMENT_DESCRIPTOR_LENGTH, PUBLISHABLE_KEY_LENGTH,
    },
    errors::{CustomResult, ParsingError, PercentageError, ValidationError},
    fp_utils::when,
};

/// Represents Percentage Value between 0 and 100 both inclusive
#[derive(Clone, Default, Debug, PartialEq, Serialize)]
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
#[derive(Clone, Debug, PartialEq, Serialize, serde::Deserialize)]
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
#[derive(Serialize, serde::Deserialize)]
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

crate::impl_to_sql_from_sql_json!(SemanticVersion);

/// Amount convertor trait for connector
pub trait AmountConvertor: Send {
    /// Output type for the connector
    type Output;
    /// helps in conversion of connector required amount type
    fn convert(
        &self,
        amount: MinorUnit,
        currency: enums::Currency,
    ) -> Result<Self::Output, error_stack::Report<ParsingError>>;

    /// helps in converting back connector required amount type to core minor unit
    fn convert_back(
        &self,
        amount: Self::Output,
        currency: enums::Currency,
    ) -> Result<MinorUnit, error_stack::Report<ParsingError>>;
}

/// Connector required amount type
#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub struct StringMinorUnitForConnector;

impl AmountConvertor for StringMinorUnitForConnector {
    type Output = StringMinorUnit;
    fn convert(
        &self,
        amount: MinorUnit,
        _currency: enums::Currency,
    ) -> Result<Self::Output, error_stack::Report<ParsingError>> {
        amount.to_minor_unit_as_string()
    }

    fn convert_back(
        &self,
        amount: Self::Output,
        _currency: enums::Currency,
    ) -> Result<MinorUnit, error_stack::Report<ParsingError>> {
        amount.to_minor_unit_as_i64()
    }
}

/// Core required conversion type
#[derive(Default, Debug, serde::Deserialize, serde::Serialize, Clone, Copy, PartialEq)]
pub struct StringMajorUnitForCore;
impl AmountConvertor for StringMajorUnitForCore {
    type Output = StringMajorUnit;
    fn convert(
        &self,
        amount: MinorUnit,
        currency: enums::Currency,
    ) -> Result<Self::Output, error_stack::Report<ParsingError>> {
        amount.to_major_unit_as_string(currency)
    }

    fn convert_back(
        &self,
        amount: StringMajorUnit,
        currency: enums::Currency,
    ) -> Result<MinorUnit, error_stack::Report<ParsingError>> {
        amount.to_minor_unit_as_i64(currency)
    }
}

/// Connector required amount type
#[derive(Default, Debug, serde::Deserialize, serde::Serialize, Clone, Copy, PartialEq)]
pub struct StringMajorUnitForConnector;

impl AmountConvertor for StringMajorUnitForConnector {
    type Output = StringMajorUnit;
    fn convert(
        &self,
        amount: MinorUnit,
        currency: enums::Currency,
    ) -> Result<Self::Output, error_stack::Report<ParsingError>> {
        amount.to_major_unit_as_string(currency)
    }

    fn convert_back(
        &self,
        amount: StringMajorUnit,
        currency: enums::Currency,
    ) -> Result<MinorUnit, error_stack::Report<ParsingError>> {
        amount.to_minor_unit_as_i64(currency)
    }
}

/// Connector required amount type
#[derive(Default, Debug, serde::Deserialize, serde::Serialize, Clone, Copy, PartialEq)]
pub struct FloatMajorUnitForConnector;

impl AmountConvertor for FloatMajorUnitForConnector {
    type Output = FloatMajorUnit;
    fn convert(
        &self,
        amount: MinorUnit,
        currency: enums::Currency,
    ) -> Result<Self::Output, error_stack::Report<ParsingError>> {
        amount.to_major_unit_as_f64(currency)
    }
    fn convert_back(
        &self,
        amount: FloatMajorUnit,
        currency: enums::Currency,
    ) -> Result<MinorUnit, error_stack::Report<ParsingError>> {
        amount.to_minor_unit_as_i64(currency)
    }
}

/// Connector required amount type
#[derive(Default, Debug, serde::Deserialize, serde::Serialize, Clone, Copy, PartialEq)]
pub struct MinorUnitForConnector;

impl AmountConvertor for MinorUnitForConnector {
    type Output = MinorUnit;
    fn convert(
        &self,
        amount: MinorUnit,
        _currency: enums::Currency,
    ) -> Result<Self::Output, error_stack::Report<ParsingError>> {
        Ok(amount)
    }
    fn convert_back(
        &self,
        amount: MinorUnit,
        _currency: enums::Currency,
    ) -> Result<MinorUnit, error_stack::Report<ParsingError>> {
        Ok(amount)
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
    pub fn get_amount_as_i64(self) -> i64 {
        self.0
    }

    /// forms a new minor default unit i.e zero
    pub fn zero() -> Self {
        Self(0)
    }

    /// forms a new minor unit from amount
    pub fn new(value: i64) -> Self {
        Self(value)
    }

    /// Convert the amount to its major denomination based on Currency and return String
    /// Paypal Connector accepts Zero and Two decimal currency but not three decimal and it should be updated as required for 3 decimal currencies.
    /// Paypal Ref - https://developer.paypal.com/docs/reports/reference/paypal-supported-currencies/
    fn to_major_unit_as_string(
        self,
        currency: enums::Currency,
    ) -> Result<StringMajorUnit, error_stack::Report<ParsingError>> {
        let amount_f64 = self.to_major_unit_as_f64(currency)?;
        let amount_string = if currency.is_zero_decimal_currency() {
            amount_f64.0.to_string()
        } else if currency.is_three_decimal_currency() {
            format!("{:.3}", amount_f64.0)
        } else {
            format!("{:.2}", amount_f64.0)
        };
        Ok(StringMajorUnit::new(amount_string))
    }

    /// Convert the amount to its major denomination based on Currency and return f64
    fn to_major_unit_as_f64(
        self,
        currency: enums::Currency,
    ) -> Result<FloatMajorUnit, error_stack::Report<ParsingError>> {
        let amount_decimal =
            Decimal::from_i64(self.0).ok_or(ParsingError::I64ToDecimalConversionFailure)?;

        let amount = if currency.is_zero_decimal_currency() {
            amount_decimal
        } else if currency.is_three_decimal_currency() {
            amount_decimal / Decimal::from(1000)
        } else {
            amount_decimal / Decimal::from(100)
        };
        let amount_f64 = amount
            .to_f64()
            .ok_or(ParsingError::FloatToDecimalConversionFailure)?;
        Ok(FloatMajorUnit::new(amount_f64))
    }

    ///Convert minor unit to string minor unit
    fn to_minor_unit_as_string(self) -> Result<StringMinorUnit, error_stack::Report<ParsingError>> {
        Ok(StringMinorUnit::new(self.0.to_string()))
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

impl Mul<u16> for MinorUnit {
    type Output = Self;

    fn mul(self, a2: u16) -> Self::Output {
        Self(self.0 * i64::from(a2))
    }
}

impl Sum for MinorUnit {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self(0), |a, b| a + b)
    }
}

/// Connector specific types to send
#[derive(Default, Debug, serde::Deserialize, serde::Serialize, Clone, PartialEq)]
pub struct StringMinorUnit(String);

impl StringMinorUnit {
    /// forms a new minor unit in string from amount
    fn new(value: String) -> Self {
        Self(value)
    }

    /// converts to minor unit i64 from minor unit string value
    fn to_minor_unit_as_i64(&self) -> Result<MinorUnit, error_stack::Report<ParsingError>> {
        let amount_string = &self.0;
        let amount_decimal = Decimal::from_str(amount_string).map_err(|e| {
            ParsingError::StringToDecimalConversionFailure {
                error: e.to_string(),
            }
        })?;
        let amount_i64 = amount_decimal
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
    fn new(value: f64) -> Self {
        Self(value)
    }

    /// forms a new major unit with zero amount
    pub fn zero() -> Self {
        Self(0.0)
    }

    /// converts to minor unit as i64 from FloatMajorUnit
    fn to_minor_unit_as_i64(
        self,
        currency: enums::Currency,
    ) -> Result<MinorUnit, error_stack::Report<ParsingError>> {
        let amount_decimal =
            Decimal::from_f64(self.0).ok_or(ParsingError::FloatToDecimalConversionFailure)?;

        let amount = if currency.is_zero_decimal_currency() {
            amount_decimal
        } else if currency.is_three_decimal_currency() {
            amount_decimal * Decimal::from(1000)
        } else {
            amount_decimal * Decimal::from(100)
        };

        let amount_i64 = amount
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
    fn new(value: String) -> Self {
        Self(value)
    }

    /// Converts to minor unit as i64 from StringMajorUnit
    fn to_minor_unit_as_i64(
        &self,
        currency: enums::Currency,
    ) -> Result<MinorUnit, error_stack::Report<ParsingError>> {
        let amount_decimal = Decimal::from_str(&self.0).map_err(|e| {
            ParsingError::StringToDecimalConversionFailure {
                error: e.to_string(),
            }
        })?;

        let amount = if currency.is_zero_decimal_currency() {
            amount_decimal
        } else if currency.is_three_decimal_currency() {
            amount_decimal * Decimal::from(1000)
        } else {
            amount_decimal * Decimal::from(100)
        };
        let amount_i64 = amount
            .to_i64()
            .ok_or(ParsingError::DecimalToI64ConversionFailure)?;
        Ok(MinorUnit::new(amount_i64))
    }
    /// forms a new StringMajorUnit default unit i.e zero
    pub fn zero() -> Self {
        Self("0".to_string())
    }
    /// Get string amount from struct to be removed in future
    pub fn get_amount_as_string(&self) -> String {
        self.0.clone()
    }
}

#[derive(
    Debug,
    serde::Deserialize,
    AsExpression,
    serde::Serialize,
    Clone,
    PartialEq,
    Eq,
    Hash,
    ToSchema,
    PartialOrd,
)]
#[diesel(sql_type = sql_types::Text)]
/// This domain type can be used for any url
pub struct Url(url::Url);

impl Url {
    /// Get string representation of the url
    pub fn get_string_repr(&self) -> &str {
        self.0.as_str()
    }

    /// wrap the url::Url in Url type
    pub fn wrap(url: url::Url) -> Self {
        Self(url)
    }

    /// Get the inner url
    pub fn into_inner(self) -> url::Url {
        self.0
    }

    /// Add query params to the url
    pub fn add_query_params(mut self, (key, value): (&str, &str)) -> Self {
        let url = self
            .0
            .query_pairs_mut()
            .append_pair(key, value)
            .finish()
            .clone();
        Self(url)
    }
}

impl<DB> ToSql<sql_types::Text, DB> for Url
where
    DB: Backend,
    str: ToSql<sql_types::Text, DB>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> diesel::serialize::Result {
        let url_string = self.0.as_str();
        url_string.to_sql(out)
    }
}

impl<DB> FromSql<sql_types::Text, DB> for Url
where
    DB: Backend,
    String: FromSql<sql_types::Text, DB>,
{
    fn from_sql(value: DB::RawValue<'_>) -> deserialize::Result<Self> {
        let val = String::from_sql(value)?;
        let url = url::Url::parse(&val)?;
        Ok(Self(url))
    }
}

#[cfg(feature = "v2")]
pub use client_secret_type::ClientSecret;
#[cfg(feature = "v2")]
mod client_secret_type {
    use std::fmt;

    use masking::PeekInterface;
    use router_env::logger;

    use super::*;
    use crate::id_type;

    /// A domain type that can be used to represent a client secret
    /// Client secret is generated for a payment and is used to authenticate the client side api calls
    #[derive(Debug, PartialEq, Clone, AsExpression)]
    #[diesel(sql_type = sql_types::Text)]
    pub struct ClientSecret {
        /// The payment id of the payment
        pub payment_id: id_type::GlobalPaymentId,
        /// The secret string
        pub secret: masking::Secret<String>,
    }

    impl ClientSecret {
        pub(crate) fn get_string_repr(&self) -> String {
            format!(
                "{}_secret_{}",
                self.payment_id.get_string_repr(),
                self.secret.peek()
            )
        }

        /// Create a new client secret
        pub(crate) fn new(payment_id: id_type::GlobalPaymentId, secret: String) -> Self {
            Self {
                payment_id,
                secret: masking::Secret::new(secret),
            }
        }
    }

    impl FromStr for ClientSecret {
        type Err = ParsingError;

        fn from_str(str_value: &str) -> Result<Self, Self::Err> {
            let (payment_id, secret) =
                str_value
                    .rsplit_once("_secret_")
                    .ok_or(ParsingError::EncodeError(
                        "Expected a string in the format '{payment_id}_secret_{secret}'",
                    ))?;

            let payment_id = id_type::GlobalPaymentId::try_from(Cow::Owned(payment_id.to_owned()))
                .map_err(|err| {
                    logger::error!(global_payment_id_error=?err);
                    ParsingError::EncodeError("Error while constructing GlobalPaymentId")
                })?;

            Ok(Self {
                payment_id,
                secret: masking::Secret::new(secret.to_owned()),
            })
        }
    }

    impl<'de> Deserialize<'de> for ClientSecret {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            struct ClientSecretVisitor;

            impl Visitor<'_> for ClientSecretVisitor {
                type Value = ClientSecret;

                fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                    formatter.write_str("a string in the format '{payment_id}_secret_{secret}'")
                }

                fn visit_str<E>(self, value: &str) -> Result<ClientSecret, E>
                where
                    E: serde::de::Error,
                {
                    let (payment_id, secret) = value.rsplit_once("_secret_").ok_or_else(|| {
                        E::invalid_value(
                            serde::de::Unexpected::Str(value),
                            &"a string with '_secret_'",
                        )
                    })?;

                    let payment_id =
                        id_type::GlobalPaymentId::try_from(Cow::Owned(payment_id.to_owned()))
                            .map_err(serde::de::Error::custom)?;

                    Ok(ClientSecret {
                        payment_id,
                        secret: masking::Secret::new(secret.to_owned()),
                    })
                }
            }

            deserializer.deserialize_str(ClientSecretVisitor)
        }
    }

    impl Serialize for ClientSecret {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::ser::Serializer,
        {
            serializer.serialize_str(self.get_string_repr().as_str())
        }
    }

    impl ToSql<sql_types::Text, diesel::pg::Pg> for ClientSecret
    where
        String: ToSql<sql_types::Text, diesel::pg::Pg>,
    {
        fn to_sql<'b>(
            &'b self,
            out: &mut Output<'b, '_, diesel::pg::Pg>,
        ) -> diesel::serialize::Result {
            let string_repr = self.get_string_repr();
            <String as ToSql<sql_types::Text, diesel::pg::Pg>>::to_sql(
                &string_repr,
                &mut out.reborrow(),
            )
        }
    }

    impl<DB> FromSql<sql_types::Text, DB> for ClientSecret
    where
        DB: Backend,
        String: FromSql<sql_types::Text, DB>,
    {
        fn from_sql(value: DB::RawValue<'_>) -> deserialize::Result<Self> {
            let string_repr = String::from_sql(value)?;
            let (payment_id, secret) =
                string_repr
                    .rsplit_once("_secret_")
                    .ok_or(ParsingError::EncodeError(
                        "Expected a string in the format '{payment_id}_secret_{secret}'",
                    ))?;

            let payment_id = id_type::GlobalPaymentId::try_from(Cow::Owned(payment_id.to_owned()))
                .map_err(|err| {
                    logger::error!(global_payment_id_error=?err);
                    ParsingError::EncodeError("Error while constructing GlobalPaymentId")
                })?;

            Ok(Self {
                payment_id,
                secret: masking::Secret::new(secret.to_owned()),
            })
        }
    }

    impl<DB> Queryable<sql_types::Text, DB> for ClientSecret
    where
        DB: Backend,
        Self: FromSql<sql_types::Text, DB>,
    {
        type Row = Self;

        fn build(row: Self::Row) -> deserialize::Result<Self> {
            Ok(row)
        }
    }
    crate::impl_serializable_secret_id_type!(ClientSecret);
    #[cfg(test)]
    mod client_secret_tests {
        #![allow(clippy::expect_used)]
        #![allow(clippy::unwrap_used)]

        use serde_json;

        use super::*;
        use crate::id_type::GlobalPaymentId;

        #[test]
        fn test_serialize_client_secret() {
            let global_payment_id = "12345_pay_1a961ed9093c48b09781bf8ab17ba6bd";
            let secret = "fc34taHLw1ekPgNh92qr".to_string();

            let expected_client_secret_string = format!("\"{global_payment_id}_secret_{secret}\"");

            let client_secret1 = ClientSecret {
                payment_id: GlobalPaymentId::try_from(Cow::Borrowed(global_payment_id)).unwrap(),
                secret: masking::Secret::new(secret),
            };

            let parsed_client_secret =
                serde_json::to_string(&client_secret1).expect("Failed to serialize client_secret1");

            assert_eq!(expected_client_secret_string, parsed_client_secret);
        }

        #[test]
        fn test_deserialize_client_secret() {
            // This is a valid global id
            let global_payment_id_str = "12345_pay_1a961ed9093c48b09781bf8ab17ba6bd";
            let secret = "fc34taHLw1ekPgNh92qr".to_string();

            let valid_payment_global_id =
                GlobalPaymentId::try_from(Cow::Borrowed(global_payment_id_str))
                    .expect("Failed to create valid global payment id");

            // This is an invalid global id because of the cell id being in invalid length
            let invalid_global_payment_id = "123_pay_1a961ed9093c48b09781bf8ab17ba6bd";

            // Create a client secret string which is valid
            let valid_client_secret = format!(r#""{global_payment_id_str}_secret_{secret}""#);

            dbg!(&valid_client_secret);

            // Create a client secret string which is invalid
            let invalid_client_secret_because_of_invalid_payment_id =
                format!(r#""{invalid_global_payment_id}_secret_{secret}""#);

            // Create a client secret string which is invalid because of invalid secret
            let invalid_client_secret_because_of_invalid_secret =
                format!(r#""{invalid_global_payment_id}""#);

            let valid_client_secret = serde_json::from_str::<ClientSecret>(&valid_client_secret)
                .expect("Failed to deserialize client_secret_str1");

            let invalid_deser1 = serde_json::from_str::<ClientSecret>(
                &invalid_client_secret_because_of_invalid_payment_id,
            );

            dbg!(&invalid_deser1);

            let invalid_deser2 = serde_json::from_str::<ClientSecret>(
                &invalid_client_secret_because_of_invalid_secret,
            );

            dbg!(&invalid_deser2);

            assert_eq!(valid_client_secret.payment_id, valid_payment_global_id);

            assert_eq!(valid_client_secret.secret.peek(), &secret);

            assert_eq!(
                invalid_deser1.err().unwrap().to_string(),
                "Incorrect value provided for field: payment_id at line 1 column 70"
            );

            assert_eq!(
                invalid_deser2.err().unwrap().to_string(),
                "invalid value: string \"123_pay_1a961ed9093c48b09781bf8ab17ba6bd\", expected a string with '_secret_' at line 1 column 42"
            );
        }
    }
}

/// A type representing a range of time for filtering, including a mandatory start time and an optional end time.
#[derive(
    Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq, Hash, ToSchema,
)]
pub struct TimeRange {
    /// The start time to filter payments list or to get list of filters. To get list of filters start time is needed to be passed
    #[serde(with = "crate::custom_serde::iso8601")]
    #[serde(alias = "startTime")]
    pub start_time: PrimitiveDateTime,
    /// The end time to filter payments list or to get list of filters. If not passed the default time is now
    #[serde(default, with = "crate::custom_serde::iso8601::option")]
    #[serde(alias = "endTime")]
    pub end_time: Option<PrimitiveDateTime>,
}

#[cfg(test)]
mod amount_conversion_tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    const TWO_DECIMAL_CURRENCY: enums::Currency = enums::Currency::USD;
    const THREE_DECIMAL_CURRENCY: enums::Currency = enums::Currency::BHD;
    const ZERO_DECIMAL_CURRENCY: enums::Currency = enums::Currency::JPY;
    #[test]
    fn amount_conversion_to_float_major_unit() {
        let request_amount = MinorUnit::new(999999999);
        let required_conversion = FloatMajorUnitForConnector;

        // Two decimal currency conversions
        let converted_amount = required_conversion
            .convert(request_amount, TWO_DECIMAL_CURRENCY)
            .unwrap();
        assert_eq!(converted_amount.0, 9999999.99);
        let converted_back_amount = required_conversion
            .convert_back(converted_amount, TWO_DECIMAL_CURRENCY)
            .unwrap();
        assert_eq!(converted_back_amount, request_amount);

        // Three decimal currency conversions
        let converted_amount = required_conversion
            .convert(request_amount, THREE_DECIMAL_CURRENCY)
            .unwrap();
        assert_eq!(converted_amount.0, 999999.999);
        let converted_back_amount = required_conversion
            .convert_back(converted_amount, THREE_DECIMAL_CURRENCY)
            .unwrap();
        assert_eq!(converted_back_amount, request_amount);

        // Zero decimal currency conversions
        let converted_amount = required_conversion
            .convert(request_amount, ZERO_DECIMAL_CURRENCY)
            .unwrap();
        assert_eq!(converted_amount.0, 999999999.0);

        let converted_back_amount = required_conversion
            .convert_back(converted_amount, ZERO_DECIMAL_CURRENCY)
            .unwrap();
        assert_eq!(converted_back_amount, request_amount);
    }

    #[test]
    fn amount_conversion_to_string_major_unit() {
        let request_amount = MinorUnit::new(999999999);
        let required_conversion = StringMajorUnitForConnector;

        // Two decimal currency conversions
        let converted_amount_two_decimal_currency = required_conversion
            .convert(request_amount, TWO_DECIMAL_CURRENCY)
            .unwrap();
        assert_eq!(
            converted_amount_two_decimal_currency.0,
            "9999999.99".to_string()
        );
        let converted_back_amount = required_conversion
            .convert_back(converted_amount_two_decimal_currency, TWO_DECIMAL_CURRENCY)
            .unwrap();
        assert_eq!(converted_back_amount, request_amount);

        // Three decimal currency conversions
        let converted_amount_three_decimal_currency = required_conversion
            .convert(request_amount, THREE_DECIMAL_CURRENCY)
            .unwrap();
        assert_eq!(
            converted_amount_three_decimal_currency.0,
            "999999.999".to_string()
        );
        let converted_back_amount = required_conversion
            .convert_back(
                converted_amount_three_decimal_currency,
                THREE_DECIMAL_CURRENCY,
            )
            .unwrap();
        assert_eq!(converted_back_amount, request_amount);

        // Zero decimal currency conversions
        let converted_amount = required_conversion
            .convert(request_amount, ZERO_DECIMAL_CURRENCY)
            .unwrap();
        assert_eq!(converted_amount.0, "999999999".to_string());

        let converted_back_amount = required_conversion
            .convert_back(converted_amount, ZERO_DECIMAL_CURRENCY)
            .unwrap();
        assert_eq!(converted_back_amount, request_amount);
    }

    #[test]
    fn amount_conversion_to_string_minor_unit() {
        let request_amount = MinorUnit::new(999999999);
        let currency = TWO_DECIMAL_CURRENCY;
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

// Charges structs
#[derive(
    Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq, FromSqlRow, AsExpression, ToSchema,
)]
#[diesel(sql_type = Jsonb)]
/// Charge specific fields for controlling the revert of funds from either platform or connected account. Check sub-fields for more details.
pub struct ChargeRefunds {
    /// Identifier for charge created for the payment
    pub charge_id: String,

    /// Toggle for reverting the application fee that was collected for the payment.
    /// If set to false, the funds are pulled from the destination account.
    pub revert_platform_fee: Option<bool>,

    /// Toggle for reverting the transfer that was made during the charge.
    /// If set to false, the funds are pulled from the main platform's account.
    pub revert_transfer: Option<bool>,
}

crate::impl_to_sql_from_sql_json!(ChargeRefunds);

/// A common type of domain type that can be used for fields that contain a string with restriction of length
#[derive(Debug, Clone, Serialize, Hash, PartialEq, Eq, AsExpression)]
#[diesel(sql_type = sql_types::Text)]
pub(crate) struct LengthString<const MAX_LENGTH: u16, const MIN_LENGTH: u16>(String);

/// Error generated from violation of constraints for MerchantReferenceId
#[derive(Debug, Error, PartialEq, Eq)]
pub(crate) enum LengthStringError {
    #[error("the maximum allowed length for this field is {0}")]
    /// Maximum length of string violated
    MaxLengthViolated(u16),

    #[error("the minimum required length for this field is {0}")]
    /// Minimum length of string violated
    MinLengthViolated(u16),
}

impl<const MAX_LENGTH: u16, const MIN_LENGTH: u16> LengthString<MAX_LENGTH, MIN_LENGTH> {
    /// Generates new [MerchantReferenceId] from the given input string
    pub fn from(input_string: Cow<'static, str>) -> Result<Self, LengthStringError> {
        let trimmed_input_string = input_string.trim().to_string();
        let length_of_input_string = u16::try_from(trimmed_input_string.len())
            .map_err(|_| LengthStringError::MaxLengthViolated(MAX_LENGTH))?;

        when(length_of_input_string > MAX_LENGTH, || {
            Err(LengthStringError::MaxLengthViolated(MAX_LENGTH))
        })?;

        when(length_of_input_string < MIN_LENGTH, || {
            Err(LengthStringError::MinLengthViolated(MIN_LENGTH))
        })?;

        Ok(Self(trimmed_input_string))
    }

    pub(crate) fn new_unchecked(input_string: String) -> Self {
        Self(input_string)
    }
}

impl<'de, const MAX_LENGTH: u16, const MIN_LENGTH: u16> Deserialize<'de>
    for LengthString<MAX_LENGTH, MIN_LENGTH>
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let deserialized_string = String::deserialize(deserializer)?;
        Self::from(deserialized_string.into()).map_err(serde::de::Error::custom)
    }
}

impl<DB, const MAX_LENGTH: u16, const MIN_LENGTH: u16> FromSql<sql_types::Text, DB>
    for LengthString<MAX_LENGTH, MIN_LENGTH>
where
    DB: Backend,
    String: FromSql<sql_types::Text, DB>,
{
    fn from_sql(bytes: DB::RawValue<'_>) -> deserialize::Result<Self> {
        let val = String::from_sql(bytes)?;
        Ok(Self(val))
    }
}

impl<DB, const MAX_LENGTH: u16, const MIN_LENGTH: u16> ToSql<sql_types::Text, DB>
    for LengthString<MAX_LENGTH, MIN_LENGTH>
where
    DB: Backend,
    String: ToSql<sql_types::Text, DB>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> diesel::serialize::Result {
        self.0.to_sql(out)
    }
}

impl<DB, const MAX_LENGTH: u16, const MIN_LENGTH: u16> Queryable<sql_types::Text, DB>
    for LengthString<MAX_LENGTH, MIN_LENGTH>
where
    DB: Backend,
    Self: FromSql<sql_types::Text, DB>,
{
    type Row = Self;
    fn build(row: Self::Row) -> deserialize::Result<Self> {
        Ok(row)
    }
}

/// Domain type for description
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize, AsExpression)]
#[diesel(sql_type = sql_types::Text)]
pub struct Description(LengthString<MAX_DESCRIPTION_LENGTH, 1>);

impl Description {
    /// Create a new Description Domain type without any length check from a static str
    pub fn from_str_unchecked(input_str: &'static str) -> Self {
        Self(LengthString::new_unchecked(input_str.to_owned()))
    }

    // TODO: Remove this function in future once description in router data is updated to domain type
    /// Get the string representation of the description
    pub fn get_string_repr(&self) -> &str {
        &self.0 .0
    }
}

/// Domain type for Statement Descriptor
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize, AsExpression)]
#[diesel(sql_type = sql_types::Text)]
pub struct StatementDescriptor(LengthString<MAX_STATEMENT_DESCRIPTOR_LENGTH, 1>);

impl<DB> Queryable<sql_types::Text, DB> for Description
where
    DB: Backend,
    Self: FromSql<sql_types::Text, DB>,
{
    type Row = Self;

    fn build(row: Self::Row) -> deserialize::Result<Self> {
        Ok(row)
    }
}

impl<DB> FromSql<sql_types::Text, DB> for Description
where
    DB: Backend,
    LengthString<MAX_DESCRIPTION_LENGTH, 1>: FromSql<sql_types::Text, DB>,
{
    fn from_sql(bytes: DB::RawValue<'_>) -> deserialize::Result<Self> {
        let val = LengthString::<MAX_DESCRIPTION_LENGTH, 1>::from_sql(bytes)?;
        Ok(Self(val))
    }
}

impl<DB> ToSql<sql_types::Text, DB> for Description
where
    DB: Backend,
    LengthString<MAX_DESCRIPTION_LENGTH, 1>: ToSql<sql_types::Text, DB>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> diesel::serialize::Result {
        self.0.to_sql(out)
    }
}

impl<DB> Queryable<sql_types::Text, DB> for StatementDescriptor
where
    DB: Backend,
    Self: FromSql<sql_types::Text, DB>,
{
    type Row = Self;

    fn build(row: Self::Row) -> deserialize::Result<Self> {
        Ok(row)
    }
}

impl<DB> FromSql<sql_types::Text, DB> for StatementDescriptor
where
    DB: Backend,
    LengthString<MAX_STATEMENT_DESCRIPTOR_LENGTH, 1>: FromSql<sql_types::Text, DB>,
{
    fn from_sql(bytes: DB::RawValue<'_>) -> deserialize::Result<Self> {
        let val = LengthString::<MAX_STATEMENT_DESCRIPTOR_LENGTH, 1>::from_sql(bytes)?;
        Ok(Self(val))
    }
}

impl<DB> ToSql<sql_types::Text, DB> for StatementDescriptor
where
    DB: Backend,
    LengthString<MAX_STATEMENT_DESCRIPTOR_LENGTH, 1>: ToSql<sql_types::Text, DB>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> diesel::serialize::Result {
        self.0.to_sql(out)
    }
}

/// Domain type for unified code
#[derive(
    Debug, Clone, PartialEq, Eq, Queryable, serde::Deserialize, serde::Serialize, AsExpression,
)]
#[diesel(sql_type = sql_types::Text)]
pub struct UnifiedCode(pub String);

impl TryFrom<String> for UnifiedCode {
    type Error = error_stack::Report<ValidationError>;
    fn try_from(src: String) -> Result<Self, Self::Error> {
        if src.len() > 255 {
            Err(report!(ValidationError::InvalidValue {
                message: "unified_code's length should not exceed 255 characters".to_string()
            }))
        } else {
            Ok(Self(src))
        }
    }
}

impl<DB> Queryable<sql_types::Text, DB> for UnifiedCode
where
    DB: Backend,
    Self: FromSql<sql_types::Text, DB>,
{
    type Row = Self;

    fn build(row: Self::Row) -> deserialize::Result<Self> {
        Ok(row)
    }
}
impl<DB> FromSql<sql_types::Text, DB> for UnifiedCode
where
    DB: Backend,
    String: FromSql<sql_types::Text, DB>,
{
    fn from_sql(bytes: DB::RawValue<'_>) -> deserialize::Result<Self> {
        let val = String::from_sql(bytes)?;
        Ok(Self::try_from(val)?)
    }
}

impl<DB> ToSql<sql_types::Text, DB> for UnifiedCode
where
    DB: Backend,
    String: ToSql<sql_types::Text, DB>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> diesel::serialize::Result {
        self.0.to_sql(out)
    }
}

/// Domain type for unified messages
#[derive(
    Debug, Clone, PartialEq, Eq, Queryable, serde::Deserialize, serde::Serialize, AsExpression,
)]
#[diesel(sql_type = sql_types::Text)]
pub struct UnifiedMessage(pub String);

impl TryFrom<String> for UnifiedMessage {
    type Error = error_stack::Report<ValidationError>;
    fn try_from(src: String) -> Result<Self, Self::Error> {
        if src.len() > 1024 {
            Err(report!(ValidationError::InvalidValue {
                message: "unified_message's length should not exceed 1024 characters".to_string()
            }))
        } else {
            Ok(Self(src))
        }
    }
}

impl<DB> Queryable<sql_types::Text, DB> for UnifiedMessage
where
    DB: Backend,
    Self: FromSql<sql_types::Text, DB>,
{
    type Row = Self;

    fn build(row: Self::Row) -> deserialize::Result<Self> {
        Ok(row)
    }
}
impl<DB> FromSql<sql_types::Text, DB> for UnifiedMessage
where
    DB: Backend,
    String: FromSql<sql_types::Text, DB>,
{
    fn from_sql(bytes: DB::RawValue<'_>) -> deserialize::Result<Self> {
        let val = String::from_sql(bytes)?;
        Ok(Self::try_from(val)?)
    }
}

impl<DB> ToSql<sql_types::Text, DB> for UnifiedMessage
where
    DB: Backend,
    String: ToSql<sql_types::Text, DB>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> diesel::serialize::Result {
        self.0.to_sql(out)
    }
}

#[cfg(feature = "v2")]
/// Browser information to be used for 3DS 2.0
// If any of the field is PII, then we can make them as secret
#[derive(
    ToSchema,
    Debug,
    Clone,
    serde::Deserialize,
    serde::Serialize,
    Eq,
    PartialEq,
    diesel::AsExpression,
)]
#[diesel(sql_type = Jsonb)]
pub struct BrowserInformation {
    /// Color depth supported by the browser
    pub color_depth: Option<u8>,

    /// Whether java is enabled in the browser
    pub java_enabled: Option<bool>,

    /// Whether javascript is enabled in the browser
    pub java_script_enabled: Option<bool>,

    /// Language supported
    pub language: Option<String>,

    /// The screen height in pixels
    pub screen_height: Option<u32>,

    /// The screen width in pixels
    pub screen_width: Option<u32>,

    /// Time zone of the client
    pub time_zone: Option<i32>,

    /// Ip address of the client
    #[schema(value_type = Option<String>)]
    pub ip_address: Option<std::net::IpAddr>,

    /// List of headers that are accepted
    #[schema(
        example = "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,image/apng,*/*;q=0.8"
    )]
    pub accept_header: Option<String>,

    /// User-agent of the browser
    pub user_agent: Option<String>,

    /// The os type of the client device
    pub os_type: Option<String>,

    /// The os version of the client device
    pub os_version: Option<String>,

    /// The device model of the client
    pub device_model: Option<String>,

    /// Accept-language of the browser
    pub accept_language: Option<String>,
}

#[cfg(feature = "v2")]
crate::impl_to_sql_from_sql_json!(BrowserInformation);
/// Domain type for connector_transaction_id
/// Maximum length for connector's transaction_id can be 128 characters in HS DB.
/// In case connector's use an identifier whose length exceeds 128 characters,
/// the hash value of such identifiers will be stored as connector_transaction_id.
/// The actual connector's identifier will be stored in a separate column -
/// processor_transaction_data or something with a similar name.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize, AsExpression)]
#[diesel(sql_type = sql_types::Text)]
pub enum ConnectorTransactionId {
    /// Actual transaction identifier
    TxnId(String),
    /// Hashed value of the transaction identifier
    HashedData(String),
}

impl ConnectorTransactionId {
    /// Implementation for retrieving the inner identifier
    pub fn get_id(&self) -> &String {
        match self {
            Self::TxnId(id) | Self::HashedData(id) => id,
        }
    }

    /// Implementation for forming ConnectorTransactionId and an optional string to be used for connector_transaction_id and processor_transaction_data
    pub fn form_id_and_data(src: String) -> (Self, Option<String>) {
        let txn_id = Self::from(src.clone());
        match txn_id {
            Self::TxnId(_) => (txn_id, None),
            Self::HashedData(_) => (txn_id, Some(src)),
        }
    }

    /// Implementation for retrieving
    pub fn get_txn_id<'a>(
        &'a self,
        txn_data: Option<&'a String>,
    ) -> Result<&'a String, error_stack::Report<ValidationError>> {
        match (self, txn_data) {
            (Self::TxnId(id), _) => Ok(id),
            (Self::HashedData(_), Some(id)) => Ok(id),
            (Self::HashedData(id), None) => Err(report!(ValidationError::InvalidValue {
                message: "processor_transaction_data is empty for HashedData variant".to_string(),
            })
            .attach_printable(format!(
                "processor_transaction_data is empty for connector_transaction_id {}",
                id
            ))),
        }
    }
}

impl From<String> for ConnectorTransactionId {
    fn from(src: String) -> Self {
        // ID already hashed
        if src.starts_with("hs_hash_") {
            Self::HashedData(src)
        // Hash connector's transaction ID
        } else if src.len() > 128 {
            let mut hasher = blake3::Hasher::new();
            let mut output = [0u8; consts::CONNECTOR_TRANSACTION_ID_HASH_BYTES];
            hasher.update(src.as_bytes());
            hasher.finalize_xof().fill(&mut output);
            let hash = hex::encode(output);
            Self::HashedData(format!("hs_hash_{}", hash))
        // Default
        } else {
            Self::TxnId(src)
        }
    }
}

impl<DB> Queryable<sql_types::Text, DB> for ConnectorTransactionId
where
    DB: Backend,
    Self: FromSql<sql_types::Text, DB>,
{
    type Row = Self;

    fn build(row: Self::Row) -> deserialize::Result<Self> {
        Ok(row)
    }
}

impl<DB> FromSql<sql_types::Text, DB> for ConnectorTransactionId
where
    DB: Backend,
    String: FromSql<sql_types::Text, DB>,
{
    fn from_sql(bytes: DB::RawValue<'_>) -> deserialize::Result<Self> {
        let val = String::from_sql(bytes)?;
        Ok(Self::from(val))
    }
}

impl<DB> ToSql<sql_types::Text, DB> for ConnectorTransactionId
where
    DB: Backend,
    String: ToSql<sql_types::Text, DB>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> diesel::serialize::Result {
        match self {
            Self::HashedData(id) | Self::TxnId(id) => id.to_sql(out),
        }
    }
}

/// Trait for fetching actual or hashed transaction IDs
pub trait ConnectorTransactionIdTrait {
    /// Returns an optional connector transaction ID
    fn get_optional_connector_transaction_id(&self) -> Option<&String> {
        None
    }
    /// Returns a connector transaction ID
    fn get_connector_transaction_id(&self) -> &String {
        self.get_optional_connector_transaction_id()
            .unwrap_or_else(|| {
                static EMPTY_STRING: String = String::new();
                &EMPTY_STRING
            })
    }
    /// Returns an optional connector refund ID
    fn get_optional_connector_refund_id(&self) -> Option<&String> {
        self.get_optional_connector_transaction_id()
    }
}

/// Domain type for PublishableKey
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize, AsExpression)]
#[diesel(sql_type = sql_types::Text)]
pub struct PublishableKey(LengthString<PUBLISHABLE_KEY_LENGTH, PUBLISHABLE_KEY_LENGTH>);

impl PublishableKey {
    /// Create a new PublishableKey Domain type without any length check from a static str
    pub fn generate(env_prefix: &'static str) -> Self {
        let publishable_key_string = format!("pk_{env_prefix}_{}", uuid::Uuid::now_v7().simple());
        Self(LengthString::new_unchecked(publishable_key_string))
    }

    /// Get the string representation of the PublishableKey
    pub fn get_string_repr(&self) -> &str {
        &self.0 .0
    }
}

impl<DB> Queryable<sql_types::Text, DB> for PublishableKey
where
    DB: Backend,
    Self: FromSql<sql_types::Text, DB>,
{
    type Row = Self;

    fn build(row: Self::Row) -> deserialize::Result<Self> {
        Ok(row)
    }
}

impl<DB> FromSql<sql_types::Text, DB> for PublishableKey
where
    DB: Backend,
    LengthString<PUBLISHABLE_KEY_LENGTH, PUBLISHABLE_KEY_LENGTH>: FromSql<sql_types::Text, DB>,
{
    fn from_sql(bytes: DB::RawValue<'_>) -> deserialize::Result<Self> {
        let val = LengthString::<PUBLISHABLE_KEY_LENGTH, PUBLISHABLE_KEY_LENGTH>::from_sql(bytes)?;
        Ok(Self(val))
    }
}

impl<DB> ToSql<sql_types::Text, DB> for PublishableKey
where
    DB: Backend,
    LengthString<PUBLISHABLE_KEY_LENGTH, PUBLISHABLE_KEY_LENGTH>: ToSql<sql_types::Text, DB>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> diesel::serialize::Result {
        self.0.to_sql(out)
    }
}

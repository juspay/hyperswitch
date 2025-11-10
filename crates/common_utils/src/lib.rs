#![warn(missing_docs, missing_debug_implementations)]
#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR" ), "/", "README.md"))]

use masking::{PeekInterface, Secret};

pub mod access_token;
pub mod consts;
pub mod crypto;
pub mod custom_serde;
#[allow(missing_docs)] // Todo: add docs
pub mod encryption;
pub mod errors;
#[allow(missing_docs)] // Todo: add docs
pub mod events;
pub mod ext_traits;
pub mod fp_utils;
/// Used for hashing
pub mod hashing;
pub mod id_type;
#[cfg(feature = "keymanager")]
pub mod keymanager;
pub mod link_utils;
pub mod macros;
#[cfg(feature = "metrics")]
pub mod metrics;
pub mod new_type;
pub mod payout_method_utils;
pub mod pii;
#[allow(missing_docs)] // Todo: add docs
pub mod request;
#[cfg(feature = "signals")]
pub mod signals;
pub mod transformers;
pub mod types;
/// Unified Connector Service (UCS) interface definitions.
///
/// This module defines types and traits for interacting with the Unified Connector Service.
/// It includes reference ID types for payments and refunds, and a trait for extracting
/// UCS reference information from requests.
pub mod ucs_types;
pub mod validation;

pub use base64_serializer::Base64Serializer;

/// Date-time utilities.
pub mod date_time {
    #[cfg(feature = "async_ext")]
    use std::time::Instant;
    use std::{marker::PhantomData, num::NonZeroU8};

    use masking::{Deserialize, Serialize};
    use time::{
        format_description::{
            well_known::iso8601::{Config, EncodedConfig, Iso8601, TimePrecision},
            BorrowedFormatItem,
        },
        OffsetDateTime, PrimitiveDateTime,
    };

    /// Enum to represent date formats
    #[derive(Debug)]
    pub enum DateFormat {
        /// Format the date in 20191105081132 format
        YYYYMMDDHHmmss,
        /// Format the date in 20191105 format
        YYYYMMDD,
        /// Format the date in 201911050811 format
        YYYYMMDDHHmm,
        /// Format the date in 05112019081132 format
        DDMMYYYYHHmmss,
    }

    /// Create a new [`PrimitiveDateTime`] with the current date and time in UTC.
    pub fn now() -> PrimitiveDateTime {
        let utc_date_time = OffsetDateTime::now_utc();
        PrimitiveDateTime::new(utc_date_time.date(), utc_date_time.time())
    }

    /// Convert from OffsetDateTime to PrimitiveDateTime
    pub fn convert_to_pdt(offset_time: OffsetDateTime) -> PrimitiveDateTime {
        PrimitiveDateTime::new(offset_time.date(), offset_time.time())
    }

    /// Return the UNIX timestamp of the current date and time in UTC
    pub fn now_unix_timestamp() -> i64 {
        OffsetDateTime::now_utc().unix_timestamp()
    }

    /// Calculate execution time for a async block in milliseconds
    #[cfg(feature = "async_ext")]
    pub async fn time_it<T, Fut: futures::Future<Output = T>, F: FnOnce() -> Fut>(
        block: F,
    ) -> (T, f64) {
        let start = Instant::now();
        let result = block().await;
        (result, start.elapsed().as_secs_f64() * 1000f64)
    }

    /// Return the given date and time in UTC with the given format Eg: format: YYYYMMDDHHmmss Eg: 20191105081132
    pub fn format_date(
        date: PrimitiveDateTime,
        format: DateFormat,
    ) -> Result<String, time::error::Format> {
        let format = <&[BorrowedFormatItem<'_>]>::from(format);
        date.format(&format)
    }

    /// Return the current date and time in UTC with the format [year]-[month]-[day]T[hour]:[minute]:[second].mmmZ Eg: 2023-02-15T13:33:18.898Z
    pub fn date_as_yyyymmddthhmmssmmmz() -> Result<String, time::error::Format> {
        const ISO_CONFIG: EncodedConfig = Config::DEFAULT
            .set_time_precision(TimePrecision::Second {
                decimal_digits: NonZeroU8::new(3),
            })
            .encode();
        now().assume_utc().format(&Iso8601::<ISO_CONFIG>)
    }

    /// Return the current date and time in UTC formatted as "ddd, DD MMM YYYY HH:mm:ss GMT".
    pub fn now_rfc7231_http_date() -> Result<String, time::error::Format> {
        let now_utc = OffsetDateTime::now_utc();
        // Desired format: ddd, DD MMM YYYY HH:mm:ss GMT
        // Example: Fri, 23 May 2025 06:19:35 GMT
        let format = time::macros::format_description!(
            "[weekday repr:short], [day padding:zero] [month repr:short] [year repr:full] [hour padding:zero repr:24]:[minute padding:zero]:[second padding:zero] GMT"
        );
        now_utc.format(&format)
    }

    impl From<DateFormat> for &[BorrowedFormatItem<'_>] {
        fn from(format: DateFormat) -> Self {
            match format {
                DateFormat::YYYYMMDDHHmmss => time::macros::format_description!("[year repr:full][month padding:zero repr:numerical][day padding:zero][hour padding:zero repr:24][minute padding:zero][second padding:zero]"),
                DateFormat::YYYYMMDD => time::macros::format_description!("[year repr:full][month padding:zero repr:numerical][day padding:zero]"),
                DateFormat::YYYYMMDDHHmm => time::macros::format_description!("[year repr:full][month padding:zero repr:numerical][day padding:zero][hour padding:zero repr:24][minute padding:zero]"),
                DateFormat::DDMMYYYYHHmmss => time::macros::format_description!("[day padding:zero][month padding:zero repr:numerical][year repr:full][hour padding:zero repr:24][minute padding:zero][second padding:zero]"),
            }
        }
    }

    /// Format the date in 05112019 format
    #[derive(Debug, Clone)]
    pub struct DDMMYYYY;
    /// Format the date in 20191105 format
    #[derive(Debug, Clone)]
    pub struct YYYYMMDD;
    /// Format the date in 20191105081132 format
    #[derive(Debug, Clone)]
    pub struct YYYYMMDDHHmmss;

    /// To serialize the date in Dateformats like YYYYMMDDHHmmss, YYYYMMDD, DDMMYYYY
    #[derive(Debug, Deserialize, Clone)]
    pub struct DateTime<T: TimeStrategy> {
        inner: PhantomData<T>,
        value: PrimitiveDateTime,
    }

    impl<T: TimeStrategy> From<PrimitiveDateTime> for DateTime<T> {
        fn from(value: PrimitiveDateTime) -> Self {
            Self {
                inner: PhantomData,
                value,
            }
        }
    }

    /// Time strategy for the Date, Eg: YYYYMMDDHHmmss, YYYYMMDD, DDMMYYYY
    pub trait TimeStrategy {
        /// Stringify the date as per the Time strategy
        fn fmt(input: &PrimitiveDateTime, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result;
    }

    impl<T: TimeStrategy> Serialize for DateTime<T> {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            serializer.collect_str(self)
        }
    }

    impl<T: TimeStrategy> std::fmt::Display for DateTime<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            T::fmt(&self.value, f)
        }
    }

    impl TimeStrategy for DDMMYYYY {
        fn fmt(input: &PrimitiveDateTime, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let year = input.year();
            #[allow(clippy::as_conversions)]
            let month = input.month() as u8;
            let day = input.day();
            let output = format!("{day:02}{month:02}{year}");
            f.write_str(&output)
        }
    }

    impl TimeStrategy for YYYYMMDD {
        fn fmt(input: &PrimitiveDateTime, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let year = input.year();
            #[allow(clippy::as_conversions)]
            let month: u8 = input.month() as u8;
            let day = input.day();
            let output = format!("{year}{month:02}{day:02}");
            f.write_str(&output)
        }
    }

    impl TimeStrategy for YYYYMMDDHHmmss {
        fn fmt(input: &PrimitiveDateTime, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let year = input.year();
            #[allow(clippy::as_conversions)]
            let month = input.month() as u8;
            let day = input.day();
            let hour = input.hour();
            let minute = input.minute();
            let second = input.second();
            let output = format!("{year}{month:02}{day:02}{hour:02}{minute:02}{second:02}");
            f.write_str(&output)
        }
    }
}

/// Generate a nanoid with the given prefix and length
#[inline]
pub fn generate_id(length: usize, prefix: &str) -> String {
    format!("{}_{}", prefix, nanoid::nanoid!(length, &consts::ALPHABETS))
}

/// Generate a ReferenceId with the default length with the given prefix
fn generate_ref_id_with_default_length<const MAX_LENGTH: u8, const MIN_LENGTH: u8>(
    prefix: &str,
) -> id_type::LengthId<MAX_LENGTH, MIN_LENGTH> {
    id_type::LengthId::<MAX_LENGTH, MIN_LENGTH>::new(prefix)
}

/// Generate a customer id with default length, with prefix as `cus`
pub fn generate_customer_id_of_default_length() -> id_type::CustomerId {
    use id_type::GenerateId;

    id_type::CustomerId::generate()
}

/// Generate a organization id with default length, with prefix as `org`
pub fn generate_organization_id_of_default_length() -> id_type::OrganizationId {
    use id_type::GenerateId;

    id_type::OrganizationId::generate()
}

/// Generate a profile id with default length, with prefix as `pro`
pub fn generate_profile_id_of_default_length() -> id_type::ProfileId {
    use id_type::GenerateId;

    id_type::ProfileId::generate()
}

/// Generate a routing id with default length, with prefix as `routing`
pub fn generate_routing_id_of_default_length() -> id_type::RoutingId {
    use id_type::GenerateId;

    id_type::RoutingId::generate()
}
/// Generate a merchant_connector_account id with default length, with prefix as `mca`
pub fn generate_merchant_connector_account_id_of_default_length(
) -> id_type::MerchantConnectorAccountId {
    use id_type::GenerateId;

    id_type::MerchantConnectorAccountId::generate()
}

/// Generate a profile_acquirer id with default length, with prefix as `mer_acq`
pub fn generate_profile_acquirer_id_of_default_length() -> id_type::ProfileAcquirerId {
    use id_type::GenerateId;

    id_type::ProfileAcquirerId::generate()
}

/// Generate a nanoid with the given prefix and a default length
#[inline]
pub fn generate_id_with_default_len(prefix: &str) -> String {
    let len: usize = consts::ID_LENGTH;
    format!("{}_{}", prefix, nanoid::nanoid!(len, &consts::ALPHABETS))
}

/// Generate a time-ordered (time-sortable) unique identifier using the current time
#[inline]
pub fn generate_time_ordered_id(prefix: &str) -> String {
    format!("{prefix}_{}", uuid::Uuid::now_v7().as_simple())
}

/// Generate a time-ordered (time-sortable) unique identifier using the current time without prefix
#[inline]
pub fn generate_time_ordered_id_without_prefix() -> String {
    uuid::Uuid::now_v7().as_simple().to_string()
}

/// Generate a nanoid with the specified length
#[inline]
pub fn generate_id_with_len(length: usize) -> String {
    nanoid::nanoid!(length, &consts::ALPHABETS)
}
#[allow(missing_docs)]
pub trait DbConnectionParams {
    fn get_username(&self) -> &str;
    fn get_password(&self) -> Secret<String>;
    fn get_host(&self) -> &str;
    fn get_port(&self) -> u16;
    fn get_dbname(&self) -> &str;
    fn get_database_url(&self, schema: &str) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}?application_name={}&options=-c%20search_path%3D{}",
            self.get_username(),
            self.get_password().peek(),
            self.get_host(),
            self.get_port(),
            self.get_dbname(),
            schema,
            schema,
        )
    }
}

// Can't add doc comments for macro invocations, neither does the macro allow it.
#[allow(missing_docs)]
mod base64_serializer {
    use base64_serde::base64_serde_type;

    base64_serde_type!(pub Base64Serializer, crate::consts::BASE64_ENGINE);
}

#[cfg(test)]
mod nanoid_tests {
    use super::*;
    use crate::{
        consts::{
            MAX_ALLOWED_MERCHANT_REFERENCE_ID_LENGTH, MIN_REQUIRED_MERCHANT_REFERENCE_ID_LENGTH,
        },
        id_type::AlphaNumericId,
    };

    #[test]
    fn test_generate_id_with_alphanumeric_id() {
        let alphanumeric_id = AlphaNumericId::from(generate_id(10, "def").into());
        assert!(alphanumeric_id.is_ok())
    }

    #[test]
    fn test_generate_merchant_ref_id_with_default_length() {
        let ref_id = id_type::LengthId::<
            MAX_ALLOWED_MERCHANT_REFERENCE_ID_LENGTH,
            MIN_REQUIRED_MERCHANT_REFERENCE_ID_LENGTH,
        >::from(generate_id_with_default_len("def").into());

        assert!(ref_id.is_ok())
    }
}

/// Module for tokenization-related functionality
///
/// This module provides types and functions for handling tokenized payment data,
/// including response structures and token generation utilities.
#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
pub mod tokenization;

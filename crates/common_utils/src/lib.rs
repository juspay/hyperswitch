#![forbid(unsafe_code)]
#![warn(missing_docs, missing_debug_implementations)]
#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR" ), "/", "README.md"))]

pub mod consts;
pub mod crypto;
pub mod custom_serde;
pub mod errors;
#[allow(missing_docs)] // Todo: add docs
pub mod events;
pub mod ext_traits;
pub mod fp_utils;
pub mod macros;
pub mod pii;
#[allow(missing_docs)] // Todo: add docs
pub mod request;
#[cfg(feature = "signals")]
pub mod signals;
#[allow(missing_docs)] // Todo: add docs
pub mod static_cache;
pub mod types;
pub mod validation;

/// Date-time utilities.
pub mod date_time {
    use std::{marker::PhantomData, num::NonZeroU8};

    use masking::{Deserialize, Serialize};
    #[cfg(feature = "async_ext")]
    use time::Instant;
    use time::{
        format_description::{
            well_known::iso8601::{Config, EncodedConfig, Iso8601, TimePrecision},
            FormatItem,
        },
        OffsetDateTime, PrimitiveDateTime,
    };
    /// Struct to represent milliseconds in time sensitive data fields
    #[derive(Debug)]
    pub struct Milliseconds(i32);

    /// Enum to represent date formats
    #[derive(Debug)]
    pub enum DateFormat {
        /// Format the date in 20191105081132 format
        YYYYMMDDHHmmss,
        /// Format the date in 20191105 format
        YYYYMMDD,
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
        /// Asynchronously measures the time taken for a given asynchronous operation to complete
    ///
    /// # Arguments
    ///
    /// * `block` - A closure representing the asynchronous operation to be timed
    ///
    /// # Returns
    ///
    /// A tuple containing the result of the asynchronous operation and the time taken in milliseconds
    ///
    pub async fn time_it<T, Fut: futures::Future<Output = T>, F: FnOnce() -> Fut>(
        block: F,
    ) -> (T, f64) {
        let start = Instant::now();
        let result = block().await;
        (result, start.elapsed().as_seconds_f64() * 1000f64)
    }

    /// Return the given date and time in UTC with the given format Eg: format: YYYYMMDDHHmmss Eg: 20191105081132
    pub fn format_date(
        date: PrimitiveDateTime,
        format: DateFormat,
    ) -> Result<String, time::error::Format> {
        let format = <&[FormatItem<'_>]>::from(format);
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

    impl From<DateFormat> for &[FormatItem<'_>] {
                /// This method takes a DateFormat enum and returns a format description string based on the input format.
        fn from(format: DateFormat) -> Self {
            match format {
                DateFormat::YYYYMMDDHHmmss => time::macros::format_description!("[year repr:full][month padding:zero repr:numerical][day padding:zero][hour padding:zero repr:24][minute padding:zero][second padding:zero]"),
                DateFormat::YYYYMMDD => time::macros::format_description!("[year repr:full][month padding:zero repr:numerical][day padding:zero]"),
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
                /// Creates a new instance of Self from a given PrimitiveDateTime.
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
                /// Serializes the object using the provided serializer.
        /// 
        /// This method takes a reference to the object and a serializer, and attempts to serialize the object using the provided serializer. It returns a Result containing the serialized output if successful, or an error if serialization fails.
        /// 
        /// # Arguments
        /// 
        /// * `serializer` - The serializer to use for serialization.
        /// 
        /// # Returns
        /// 
        /// A Result containing the serialized output if successful, or an error if serialization fails.
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            serializer.collect_str(self)
        }
    }

    impl<T: TimeStrategy> std::fmt::Display for DateTime<T> {
                /// Formats the value using the provided formatter.
        /// 
        /// # Arguments
        /// 
        /// * `f` - A mutable reference to a formatter that will be used to format the value.
        /// 
        /// # Returns
        /// 
        /// * `Result` - A result indicating whether the formatting was successful or not.
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            T::fmt(&self.value, f)
        }
    }

    impl TimeStrategy for DDMMYYYY {
                /// Formats the input PrimitiveDateTime into a custom date format and writes it to the provided Formatter.
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
                /// Formats the input PrimitiveDateTime as "YYYYMMDD" and writes it to the provided formatter.
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
                /// Formats the input PrimitiveDateTime into a string in the format "YYYYMMDDHHMMSS"
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
/// Generates a random ID of a specified length with a given prefix using the nanoid crate.
///
/// # Arguments
///
/// * `length` - The length of the generated ID.
/// * `prefix` - The prefix to be added to the generated ID.
///
pub fn generate_id(length: usize, prefix: &str) -> String {
    format!("{}_{}", prefix, nanoid::nanoid!(length, &consts::ALPHABETS))
}

/// Generate a nanoid with the given prefix and a default length
#[inline]
/// Generates a unique ID with a default length using the provided prefix.
///
/// # Arguments
///
/// * `prefix` - A string slice that represents the prefix for the generated ID.
///
pub fn generate_id_with_default_len(prefix: &str) -> String {
    let len = consts::ID_LENGTH;
    format!("{}_{}", prefix, nanoid::nanoid!(len, &consts::ALPHABETS))
}

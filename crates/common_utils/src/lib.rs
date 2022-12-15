#![forbid(unsafe_code)]
#![warn(
    missing_docs,
    rust_2018_idioms,
    missing_debug_implementations,
    clippy::expect_used,
    clippy::missing_panics_doc,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::panicking_unwrap,
    clippy::unreachable,
    clippy::unwrap_in_result,
    clippy::unwrap_used
)]
#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR" ), "/", "README.md"))]

pub mod consts;
pub mod custom_serde;
pub mod errors;
pub mod ext_traits;
pub mod pii;
pub mod validation;

/// Date-time utilities.
pub mod date_time {
    use time::{OffsetDateTime, PrimitiveDateTime};
    /// Struct to represent milliseconds in time sensitive data fields
    #[derive(Debug)]
    pub struct Milliseconds(i32);

    /// Create a new [`PrimitiveDateTime`] with the current date and time in UTC.
    pub fn now() -> PrimitiveDateTime {
        let utc_date_time = OffsetDateTime::now_utc();
        PrimitiveDateTime::new(utc_date_time.date(), utc_date_time.time())
    }

    /// Convert from OffsetDateTime to PrimitiveDateTime
    pub fn convert_to_pdt(offset_time: OffsetDateTime) -> PrimitiveDateTime {
        PrimitiveDateTime::new(offset_time.date(), offset_time.time())
    }
}

/// Generate a nanoid with the given prefix and length
#[inline]
pub fn generate_id(length: usize, prefix: &str) -> String {
    format!("{}_{}", prefix, nanoid::nanoid!(length, &consts::ALPHABETS))
}

/// Generate a nanoid with the given prefix and a default length
#[inline]
pub fn generate_id_with_default_len(prefix: &str) -> String {
    let len = consts::ID_LENGTH;
    format!("{}_{}", prefix, nanoid::nanoid!(len, &consts::ALPHABETS))
}

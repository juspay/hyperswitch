#![warn(missing_docs, missing_debug_implementations)]
#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR" ), "/", "README.md"))]

use hyperswitch_masking::{ExposeInterface, PeekInterface, Secret};

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
pub mod external_service;
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
pub mod request_context;
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

    use hyperswitch_masking::{Deserialize, Serialize};
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
    #[cfg_attr(feature = "deja", track_caller)]
    #[cfg_attr(
        feature = "deja",
        deja::time(component = "common_utils", operation = "date_time::now", codec = SerdeCodec,)
    )]
    pub fn now() -> PrimitiveDateTime {
        #[allow(clippy::disallowed_methods, reason = "this function IS the seam")]
        let utc_date_time = OffsetDateTime::now_utc();
        PrimitiveDateTime::new(utc_date_time.date(), utc_date_time.time())
    }

    /// Convert from OffsetDateTime to PrimitiveDateTime
    pub fn convert_to_pdt(offset_time: OffsetDateTime) -> PrimitiveDateTime {
        PrimitiveDateTime::new(offset_time.date(), offset_time.time())
    }

    /// Return the UNIX timestamp of the current date and time in UTC
    #[cfg_attr(feature = "deja", track_caller)]
    #[cfg_attr(
        feature = "deja",
        deja::time(
            component = "common_utils",
            operation = "date_time::now_unix_timestamp",
            codec = SerdeCodec,
        )
    )]
    pub fn now_unix_timestamp() -> i64 {
        #[allow(clippy::disallowed_methods, reason = "this function IS the seam")]
        OffsetDateTime::now_utc().unix_timestamp()
    }

    /// Return the UNIX timestamp in nanoseconds of the current date and time in UTC
    ///
    /// Reads the clock through `now()` rather than calling `now_utc()` again.
    /// `now()` is already an instrumented seam, so this carries no seam and no
    /// gate exception of its own, and it costs no event a caller of `now()`
    /// would not already have paid. The value is unchanged: `now()` keeps the
    /// full nanosecond, so this returns the same `i128` the direct read did.
    ///
    /// `track_caller` so that a future caller's own location, not this body,
    /// is what `now()` records.
    #[cfg_attr(feature = "deja", track_caller)]
    pub fn now_unix_timestamp_nanos() -> i128 {
        now().assume_utc().unix_timestamp_nanos()
    }

    /// Return the UNIX timestamp in milliseconds of the current date and time in UTC.
    ///
    /// Several connectors sign a millisecond timestamp into an outbound request.
    /// They open-coded `now_utc().unix_timestamp_nanos() / 1_000_000`, which
    /// reads the clock outside any seam and so cannot be reproduced on replay.
    #[cfg_attr(feature = "deja", track_caller)]
    #[cfg_attr(
        feature = "deja",
        deja::time(
            component = "common_utils",
            operation = "date_time::now_unix_timestamp_millis",
            codec = SerdeCodec,
        )
    )]
    pub fn now_unix_timestamp_millis() -> i128 {
        #[allow(
            clippy::disallowed_methods,
            reason = "this IS the seam for a millisecond timestamp"
        )]
        let now = OffsetDateTime::now_utc();
        now.unix_timestamp_nanos() / 1_000_000
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
    #[cfg_attr(feature = "deja", track_caller)]
    #[cfg_attr(
        feature = "deja",
        deja::time(
            component = "common_utils",
            operation = "date_time::date_as_yyyymmddthhmmssmmmz",
        )
    )]
    pub fn date_as_yyyymmddthhmmssmmmz() -> Result<String, time::error::Format> {
        const ISO_CONFIG: EncodedConfig = Config::DEFAULT
            .set_time_precision(TimePrecision::Second {
                decimal_digits: NonZeroU8::new(3),
            })
            .encode();
        // Read the clock directly rather than via `now()`: both are instrumented seams, and
        // nesting them would record a redundant inner event for every outer call.
        #[allow(clippy::disallowed_methods, reason = "this function IS the seam")]
        convert_to_pdt(OffsetDateTime::now_utc())
            .assume_utc()
            .format(&Iso8601::<ISO_CONFIG>)
    }

    /// Return the current date and time in UTC formatted as "ddd, DD MMM YYYY HH:mm:ss GMT".
    #[cfg_attr(feature = "deja", track_caller)]
    #[cfg_attr(
        feature = "deja",
        deja::time(
            component = "common_utils",
            operation = "date_time::now_rfc7231_http_date",
        )
    )]
    pub fn now_rfc7231_http_date() -> Result<String, time::error::Format> {
        #[allow(clippy::disallowed_methods, reason = "this function IS the seam")]
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

/// Generate a version 4 (random) UUID.
///
/// A v4 UUID is drawn entirely from the operating system's entropy, so nothing in
/// the request determines it and a replay cannot reproduce one that was read
/// raw. Connectors put this value in an idempotency key, a request reference and
/// — for deutschebank, payeezy, authipay, fiserv, fiservemea and
/// fiservcommercehub — inside the string they sign, so an unseamed read changes
/// the outbound bytes and the substitute boundary misses.
///
/// Returns the `Uuid` rather than a formatted string so that callers keep their
/// own `to_string`, `simple` or `hyphenated` rendering unchanged.
#[inline]
#[cfg_attr(feature = "deja", track_caller)]
#[cfg_attr(
    feature = "deja",
    deja::id(component = "common_utils", operation = "generate_uuid_v4", codec = SerdeCodec,)
)]
#[allow(clippy::disallowed_methods, reason = "this function IS the seam")]
pub fn generate_uuid_v4() -> uuid::Uuid {
    uuid::Uuid::new_v4()
}

/// Generate a random alphanumeric string of the given length.
///
/// This is the seam for the nonce-and-salt shape that connectors open-coded as
/// `Alphanumeric.sample_string(&mut rand::thread_rng(), n)`.
///
/// It deliberately keeps the thread-local RNG rather than routing through
/// [`crypto::generate_cryptographically_secure_random_string`], which draws from
/// `OsRng`: several of these values are signed into an outbound request, and
/// swapping the generator underneath them would be a security change riding on a
/// determinism refactor. Use the `crypto` one for anything that must be
/// unpredictable to an attacker.
#[inline]
#[cfg_attr(feature = "deja", track_caller)]
#[cfg_attr(
    feature = "deja",
    deja::id(
        component = "common_utils",
        operation = "generate_random_alphanumeric_string",
        codec = SerdeCodec,
    )
)]
pub fn generate_random_alphanumeric_string(length: usize) -> String {
    use rand::distributions::DistString;

    #[allow(clippy::disallowed_methods, reason = "this function IS the seam")]
    let mut rng = rand::thread_rng();
    rand::distributions::Alphanumeric.sample_string(&mut rng, length)
}

/// Generate a random string of decimal digits of the given length.
///
/// The digit counterpart of [`generate_random_alphanumeric_string`]; santander
/// puts both on the wire. Draws once for the whole string rather than once per
/// character, so a call costs one recorded event and not `length` of them.
#[inline]
#[cfg_attr(feature = "deja", track_caller)]
#[cfg_attr(
    feature = "deja",
    deja::id(
        component = "common_utils",
        operation = "generate_random_numeric_string",
        codec = SerdeCodec,
    )
)]
pub fn generate_random_numeric_string(length: usize) -> String {
    use rand::Rng;

    #[allow(clippy::disallowed_methods, reason = "this function IS the seam")]
    let mut rng = rand::thread_rng();
    (0..length)
        .map(|_| char::from(rng.gen_range(b'0'..=b'9')))
        .collect()
}

/// Generate a version 7 (time-ordered) UUID.
///
/// A v7 UUID is a millisecond timestamp followed by 74 random bits, so it is a
/// clock read and an entropy read at once — neither the raw-clock nor the
/// `Uuid::new_v4` gate entry catches it, which is why it has its own seam and its
/// own entry. razorpay and trustpay put one on the wire; vault ids, publishable
/// keys and relay ids are stored under one.
#[inline]
#[cfg_attr(feature = "deja", track_caller)]
#[cfg_attr(
    feature = "deja",
    deja::id(component = "common_utils", operation = "generate_uuid_v7", codec = SerdeCodec,)
)]
#[allow(clippy::disallowed_methods, reason = "this function IS the seam")]
pub fn generate_uuid_v7() -> uuid::Uuid {
    uuid::Uuid::now_v7()
}

/// Generate a nanoid of the given length using nanoid's own default alphabet.
///
/// Deliberately distinct from [`generate_id_with_len`], which uses
/// `consts::ALPHABETS` — 62 characters, no `-` or `_`. nanoid's default alphabet
/// has 64 and includes both. Five connectors put a default-alphabet nanoid on the
/// wire as a payment reference, so routing them through `generate_id_with_len`
/// would change the bytes they send; this seam keeps them byte-identical.
#[inline]
#[cfg_attr(feature = "deja", track_caller)]
#[cfg_attr(
    feature = "deja",
    deja::id(
        component = "common_utils",
        operation = "generate_nanoid_with_default_alphabet",
        codec = SerdeCodec,
    )
)]
#[allow(clippy::disallowed_macros, reason = "this function IS the seam")]
pub fn generate_nanoid_with_default_alphabet(length: usize) -> String {
    nanoid::nanoid!(length)
}

/// Draw a uniform `f64` in `[0, 1)`.
///
/// This is the seam for a sampling decision: a value drawn here is compared
/// against a rollout percentage or a firing probability, so it decides which
/// branch the request takes. Unseamed, a replay takes a different branch from
/// the recording and the divergence is nobody's bug.
#[inline]
#[cfg_attr(feature = "deja", track_caller)]
#[cfg_attr(
    feature = "deja",
    deja::id(
        component = "common_utils",
        operation = "generate_random_f64_unit",
        codec = SerdeCodec,
    )
)]
pub fn generate_random_f64_unit() -> f64 {
    use rand::Rng;

    #[allow(clippy::disallowed_methods, reason = "this function IS the seam")]
    let mut rng = rand::thread_rng();
    rng.gen_range(0.0..1.0)
}

/// Draw a uniform integer in `min..=max`, both ends inclusive.
///
/// Panics on an empty range, exactly as `rand`'s `gen_range` does for the
/// open-coded form this replaces.
#[inline]
#[cfg_attr(feature = "deja", track_caller)]
#[cfg_attr(
    feature = "deja",
    deja::id(
        component = "common_utils",
        operation = "generate_random_number_in_range",
        codec = SerdeCodec,
    )
)]
pub fn generate_random_number_in_range(min: i64, max: i64) -> i64 {
    use rand::Rng;

    #[allow(clippy::disallowed_methods, reason = "this function IS the seam")]
    let mut rng = rand::thread_rng();
    rng.gen_range(min..=max)
}

/// Pick a uniform index into a collection of `length` items, or `None` if empty.
///
/// The seam for "choose one of these at random" where the choice decides what
/// goes on the wire — which OIDC key signs a token, for instance.
#[inline]
#[cfg_attr(feature = "deja", track_caller)]
#[cfg_attr(
    feature = "deja",
    deja::id(
        component = "common_utils",
        operation = "generate_random_index",
        codec = SerdeCodec,
    )
)]
pub fn generate_random_index(length: usize) -> Option<usize> {
    use rand::Rng;

    if length == 0 {
        return None;
    }

    #[allow(clippy::disallowed_methods, reason = "this function IS the seam")]
    let mut rng = rand::thread_rng();
    Some(rng.gen_range(0..length))
}

/// The current process id.
///
/// Ambient state, not an input: a recording pod and a replay pod are different
/// processes, so anything derived from this on a request path would diverge.
/// There is no such caller today — this seam exists so that the gate on
/// `std::process::id` has somewhere to send the first one.
#[inline]
#[cfg_attr(feature = "deja", track_caller)]
#[cfg_attr(
    feature = "deja",
    deja::id(component = "common_utils", operation = "process_id", codec = SerdeCodec,)
)]
#[allow(clippy::disallowed_methods, reason = "this function IS the seam")]
pub fn process_id() -> u32 {
    std::process::id()
}

/// The host's name, read from the `HOSTNAME` environment variable.
///
/// Ambient state, like [`process_id`]: it differs between a recording host and a
/// replay host, so anything derived from it on a request path would diverge.
/// There is no such caller today; the seam exists so the gate on `gethostname`
/// has somewhere to send the first one.
///
/// Reads the environment rather than calling `gethostname(2)` on purpose. Under
/// Kubernetes `HOSTNAME` is the pod name, which is the identity that actually
/// matters here and is the same convention deja already uses to resolve its own
/// instance id (`identity.pod_name_env`). It also keeps `common_utils` free of a
/// dependency that would not build for wasm32, which this crate is compiled for.
/// Returns `None` when the variable is unset — a shell may set it without
/// exporting it, so a caller must handle its absence.
#[inline]
#[cfg_attr(feature = "deja", track_caller)]
#[cfg_attr(
    feature = "deja",
    deja::id(component = "common_utils", operation = "hostname", codec = SerdeCodec,)
)]
pub fn hostname() -> Option<String> {
    std::env::var("HOSTNAME")
        .ok()
        .filter(|name| !name.is_empty())
}

/// Produce a random permutation of `0..length`.
///
/// The seam for "shuffle these", which has no single-value shape: recording the
/// permutation costs one event where drawing it position by position would cost
/// `length` of them.
#[inline]
#[cfg_attr(feature = "deja", track_caller)]
#[cfg_attr(
    feature = "deja",
    deja::id(
        component = "common_utils",
        operation = "generate_random_permutation",
        codec = SerdeCodec,
    )
)]
pub fn generate_random_permutation(length: usize) -> Vec<usize> {
    use rand::seq::SliceRandom;

    let mut indices: Vec<usize> = (0..length).collect();

    #[allow(clippy::disallowed_methods, reason = "this function IS the seam")]
    let mut rng = rand::thread_rng();
    indices.shuffle(&mut rng);
    indices
}

/// Generate `length` random bytes.
///
/// Not cryptographically secure — it keeps the thread-local RNG the open-coded
/// callers used. For key material use
/// [`crypto::generate_cryptographically_secure_random_bytes`] instead.
#[inline]
#[cfg_attr(feature = "deja", track_caller)]
#[cfg_attr(
    feature = "deja",
    deja::id(
        component = "common_utils",
        operation = "generate_random_bytes",
        codec = SerdeCodec,
    )
)]
pub fn generate_random_bytes(length: usize) -> Vec<u8> {
    use rand::Rng;

    #[allow(clippy::disallowed_methods, reason = "this function IS the seam")]
    let mut rng = rand::thread_rng();
    (0..length).map(|_| rng.gen()).collect()
}

/// Generate a nanoid with the given prefix and length
#[inline]
#[cfg_attr(feature = "deja", track_caller)]
#[cfg_attr(
    feature = "deja",
    deja::id(component = "common_utils", operation = "generate_id", codec = SerdeCodec,)
)]
#[allow(clippy::disallowed_macros, reason = "this function IS the seam")]
pub fn generate_id(length: usize, prefix: &str) -> String {
    format!("{}_{}", prefix, nanoid::nanoid!(length, &consts::ALPHABETS))
}

/// Generate a ReferenceId with the default length with the given prefix
#[cfg_attr(feature = "deja", track_caller)]
fn generate_ref_id_with_default_length<const MAX_LENGTH: u8, const MIN_LENGTH: u8>(
    prefix: &str,
) -> id_type::LengthId<MAX_LENGTH, MIN_LENGTH> {
    id_type::LengthId::<MAX_LENGTH, MIN_LENGTH>::new(prefix)
}

/// Generate a customer id with default length, with prefix as `cus`
#[cfg_attr(feature = "deja", track_caller)]
pub fn generate_customer_id_of_default_length() -> id_type::CustomerId {
    use id_type::GenerateId;

    id_type::CustomerId::generate()
}

/// Generate a organization id with default length, with prefix as `org`
#[cfg_attr(feature = "deja", track_caller)]
pub fn generate_organization_id_of_default_length() -> id_type::OrganizationId {
    use id_type::GenerateId;

    id_type::OrganizationId::generate()
}

/// Generate a profile id with default length, with prefix as `pro`
#[cfg_attr(feature = "deja", track_caller)]
pub fn generate_profile_id_of_default_length() -> id_type::ProfileId {
    use id_type::GenerateId;

    id_type::ProfileId::generate()
}

/// Generate a routing id with default length, with prefix as `routing`
#[cfg_attr(feature = "deja", track_caller)]
pub fn generate_routing_id_of_default_length() -> id_type::RoutingId {
    use id_type::GenerateId;

    id_type::RoutingId::generate()
}
/// Generate a merchant_connector_account id with default length, with prefix as `mca`
#[cfg_attr(feature = "deja", track_caller)]
pub fn generate_merchant_connector_account_id_of_default_length(
) -> id_type::MerchantConnectorAccountId {
    use id_type::GenerateId;

    id_type::MerchantConnectorAccountId::generate()
}

/// Generate a profile_acquirer id with default length, with prefix as `mer_acq`
#[cfg_attr(feature = "deja", track_caller)]
pub fn generate_profile_acquirer_id_of_default_length() -> id_type::ProfileAcquirerId {
    use id_type::GenerateId;

    id_type::ProfileAcquirerId::generate()
}

/// Generate a nanoid with the given prefix and a default length
#[inline]
#[cfg_attr(feature = "deja", track_caller)]
#[cfg_attr(
    feature = "deja",
    deja::id(
        component = "common_utils",
        operation = "generate_id_with_default_len",
        codec = SerdeCodec,
    )
)]
#[allow(clippy::disallowed_macros, reason = "this function IS the seam")]
pub fn generate_id_with_default_len(prefix: &str) -> String {
    let len: usize = consts::ID_LENGTH;
    format!("{}_{}", prefix, nanoid::nanoid!(len, &consts::ALPHABETS))
}

/// Generate a time-ordered (time-sortable) unique identifier using the current time
#[inline]
#[cfg_attr(feature = "deja", track_caller)]
#[cfg_attr(
    feature = "deja",
    deja::id(
        component = "common_utils",
        operation = "generate_time_ordered_id",
        codec = SerdeCodec,
    )
)]
#[allow(clippy::disallowed_methods, reason = "this function IS the seam")]
pub fn generate_time_ordered_id(prefix: &str) -> String {
    format!("{prefix}_{}", uuid::Uuid::now_v7().as_simple())
}

/// Generate a time-ordered (time-sortable) unique identifier using the current time without prefix
#[inline]
#[cfg_attr(feature = "deja", track_caller)]
#[cfg_attr(
    feature = "deja",
    deja::id(
        component = "common_utils",
        operation = "generate_time_ordered_id_without_prefix",
        codec = SerdeCodec,
    )
)]
#[allow(clippy::disallowed_methods, reason = "this function IS the seam")]
pub fn generate_time_ordered_id_without_prefix() -> String {
    uuid::Uuid::now_v7().as_simple().to_string()
}

/// Generate a nanoid with the specified length
#[inline]
#[cfg_attr(feature = "deja", track_caller)]
#[cfg_attr(
    feature = "deja",
    deja::id(component = "common_utils", operation = "generate_id_with_len", codec = SerdeCodec,)
)]
#[allow(clippy::disallowed_macros, reason = "this function IS the seam")]
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

/// Merges two optional JSON values into a single JSON object.
/// If both values are objects, their key-value pairs are merged.
/// If only one value exists, it is returned.
/// If neither exists, None is returned.
pub fn merge_json_values(
    first: Option<pii::SecretSerdeValue>,
    second: Option<pii::SecretSerdeValue>,
) -> Option<pii::SecretSerdeValue> {
    match first.clone().zip(second.clone()) {
        Some((first, second)) => {
            let first_value = first.expose();
            let second_value = second.expose();

            match (first_value, second_value) {
                (
                    serde_json::Value::Object(mut first_map),
                    serde_json::Value::Object(second_map),
                ) => {
                    // if the first and second has the same keys then the value will be updated with that of the second
                    first_map.extend(second_map);
                    Some(pii::SecretSerdeValue::new(serde_json::Value::Object(
                        first_map,
                    )))
                }
                // ideally both Value should of variant Object but if one of them is not an object, it follows the previous behaviour i.e pass payment method metadata
                (first_val, _) => Some(pii::SecretSerdeValue::new(first_val)),
            }
        }
        None => first.or(second),
    }
}

pub use ext_traits::ApplyOptionField;

/// Module for tokenization-related functionality
///
/// This module provides types and functions for handling tokenized payment data,
/// including response structures and token generation utilities.
#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
pub mod tokenization;

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

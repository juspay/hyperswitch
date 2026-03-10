//! Interactions with external systems.

#![warn(missing_docs, missing_debug_implementations)]

#[cfg(feature = "aws_kms")]
pub mod aws_kms;
/// crm module
pub mod crm;
#[cfg(feature = "email")]
pub mod email;
pub mod file_storage;
/// Building grpc clients to communicate with the server
pub mod grpc_client;
#[cfg(feature = "hashicorp-vault")]
pub mod hashicorp_vault;
/// http_client module
pub mod http_client;
/// hubspot_proxy module
pub mod hubspot_proxy;
pub mod managers;
pub mod no_encryption;
#[cfg(feature = "superposition")]
pub mod superposition;
/// deserializers module_path
pub mod utils;

#[cfg(feature = "revenue_recovery")]
/// date_time module
pub mod date_time {
    use error_stack::ResultExt;

    /// Errors in time conversion
    #[derive(Debug, thiserror::Error)]
    pub enum DateTimeConversionError {
        #[error("Invalid timestamp value from prost Timestamp: out of representable range")]
        /// Error for out of range
        TimestampOutOfRange,
    }

    /// Converts a `time::PrimitiveDateTime` to a `prost_types::Timestamp`.
    pub fn convert_to_prost_timestamp(dt: time::PrimitiveDateTime) -> prost_types::Timestamp {
        let odt = dt.assume_utc();
        prost_types::Timestamp {
            seconds: odt.unix_timestamp(),
            // This conversion is safe as nanoseconds (0..999_999_999) always fit within an i32.
            #[allow(clippy::as_conversions)]
            nanos: odt.nanosecond() as i32,
        }
    }

    /// Converts a `prost_types::Timestamp` to an `time::PrimitiveDateTime`.
    pub fn convert_from_prost_timestamp(
        ts: &prost_types::Timestamp,
    ) -> error_stack::Result<time::PrimitiveDateTime, DateTimeConversionError> {
        let timestamp_nanos = i128::from(ts.seconds) * 1_000_000_000 + i128::from(ts.nanos);

        time::OffsetDateTime::from_unix_timestamp_nanos(timestamp_nanos)
            .map(|offset_dt| time::PrimitiveDateTime::new(offset_dt.date(), offset_dt.time()))
            .change_context(DateTimeConversionError::TimestampOutOfRange)
    }
}

/// Crate specific constants
pub mod consts {
    /// General purpose base64 engine
    #[cfg(feature = "aws_kms")]
    pub(crate) const BASE64_ENGINE: base64::engine::GeneralPurpose =
        base64::engine::general_purpose::STANDARD;

    /// Header key used to specify the connector name in UCS requests.
    pub(crate) const UCS_HEADER_CONNECTOR: &str = "x-connector";

    /// Header key used to indicate the authentication type being used.
    pub(crate) const UCS_HEADER_AUTH_TYPE: &str = "x-auth";

    /// Header key for sending the API key used for authentication.
    pub(crate) const UCS_HEADER_API_KEY: &str = "x-api-key";

    /// Header key for sending an additional secret key used in some auth types.
    pub(crate) const UCS_HEADER_KEY1: &str = "x-key1";

    /// Header key for sending the API secret in signature-based authentication.
    pub(crate) const UCS_HEADER_API_SECRET: &str = "x-api-secret";

    /// Header key for sending a second additional key used in multi-auth authentication.
    pub(crate) const UCS_HEADER_KEY2: &str = "x-key2";

    /// Header key for sending the AUTH KEY MAP in currency-based authentication.
    pub(crate) const UCS_HEADER_AUTH_KEY_MAP: &str = "x-auth-key-map";

    /// Header key for sending the EXTERNAL VAULT METADATA in proxy payments
    pub(crate) const UCS_HEADER_EXTERNAL_VAULT_METADATA: &str = "x-external-vault-metadata";

    /// Header key for sending the list of lineage ids
    pub(crate) const UCS_LINEAGE_IDS: &str = "x-lineage-ids";

    /// Header key for sending the merchant reference id to UCS
    pub(crate) const UCS_HEADER_REFERENCE_ID: &str = "x-reference-id";
}

/// Metrics for interactions with external systems.
#[cfg(feature = "aws_kms")]
pub mod metrics {
    use router_env::{counter_metric, global_meter, histogram_metric_f64};

    global_meter!(GLOBAL_METER, "EXTERNAL_SERVICES");

    #[cfg(feature = "aws_kms")]
    counter_metric!(AWS_KMS_DECRYPTION_FAILURES, GLOBAL_METER); // No. of AWS KMS Decryption failures
    #[cfg(feature = "aws_kms")]
    counter_metric!(AWS_KMS_ENCRYPTION_FAILURES, GLOBAL_METER); // No. of AWS KMS Encryption failures

    #[cfg(feature = "aws_kms")]
    histogram_metric_f64!(AWS_KMS_DECRYPT_TIME, GLOBAL_METER); // Histogram for AWS KMS decryption time (in sec)
    #[cfg(feature = "aws_kms")]
    histogram_metric_f64!(AWS_KMS_ENCRYPT_TIME, GLOBAL_METER); // Histogram for AWS KMS encryption time (in sec)
}

//! Interactions with external systems.

#![warn(missing_docs, missing_debug_implementations)]

#[cfg(feature = "email")]
pub mod email;

#[cfg(feature = "aws_kms")]
pub mod aws_kms;

pub mod file_storage;
#[cfg(feature = "hashicorp-vault")]
pub mod hashicorp_vault;

pub mod no_encryption;

/// Building grpc clients to communicate with the server
pub mod grpc_client;

/// http_client module
pub mod http_client;

/// hubspot_proxy module
pub mod hubspot_proxy;

pub mod managers;

/// crm module
pub mod crm;

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

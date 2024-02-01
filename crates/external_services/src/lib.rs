//! Interactions with external systems.

#![forbid(unsafe_code)]
#![warn(missing_docs, missing_debug_implementations)]

#[cfg(feature = "email")]
pub mod email;

#[cfg(feature = "kms")]
pub mod kms;

pub mod file_storage;
#[cfg(feature = "hashicorp-vault")]
pub mod hashicorp_vault;

/// Crate specific constants
#[cfg(feature = "kms")]
pub mod consts {
    /// General purpose base64 engine
    pub(crate) const BASE64_ENGINE: base64::engine::GeneralPurpose =
        base64::engine::general_purpose::STANDARD;
}

/// Metrics for interactions with external systems.
#[cfg(feature = "kms")]
pub mod metrics {
    use router_env::{counter_metric, global_meter, histogram_metric, metrics_context};

    metrics_context!(CONTEXT);
    global_meter!(GLOBAL_METER, "EXTERNAL_SERVICES");

    #[cfg(feature = "kms")]
    counter_metric!(AWS_KMS_DECRYPTION_FAILURES, GLOBAL_METER); // No. of AWS KMS Decryption failures
    #[cfg(feature = "kms")]
    counter_metric!(AWS_KMS_ENCRYPTION_FAILURES, GLOBAL_METER); // No. of AWS KMS Encryption failures

    #[cfg(feature = "kms")]
    histogram_metric!(AWS_KMS_DECRYPT_TIME, GLOBAL_METER); // Histogram for KMS decryption time (in sec)
    #[cfg(feature = "kms")]
    histogram_metric!(AWS_KMS_ENCRYPT_TIME, GLOBAL_METER); // Histogram for KMS encryption time (in sec)
}

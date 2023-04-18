//! Interactions with external systems.

#![forbid(unsafe_code)]
#![warn(missing_docs, missing_debug_implementations)]

#[cfg(feature = "kms")]
pub mod kms;

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
    use router_env::{counter_metric, global_meter, metrics_context};

    metrics_context!(CONTEXT);
    global_meter!(GLOBAL_METER, "EXTERNAL_SERVICES");

    #[cfg(feature = "kms")]
    counter_metric!(AWS_KMS_FAILURES, GLOBAL_METER); // No. of AWS KMS API failures
}

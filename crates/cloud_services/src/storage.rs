//! Object storage clients.

#[cfg(feature = "gcs_object_storage")]
pub mod gcs;
#[cfg(feature = "oci_object_storage")]
pub mod oci;
#[cfg(feature = "s3")]
pub mod s3;

/// A storage configuration value that is missing or malformed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0}")]
pub struct InvalidStorageConfig(&'static str);

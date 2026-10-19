//! Object storage clients.

#[cfg(feature = "gcs_object_storage")]
pub mod gcs;
#[cfg(feature = "oci_object_storage")]
pub mod oci;
#[cfg(feature = "s3")]
pub mod s3;

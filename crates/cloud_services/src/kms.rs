//! Key management service clients.

/// OCI Vault KMS client.
///
/// Re-exported from the standalone `oci_kms` crate, which other services already depend on by
/// git, so it stays in place rather than moving here.
#[cfg(feature = "oci_kms")]
pub mod oci {
    pub use oci_kms::{
        DataKey, Environment, OciKmsClient, OciKmsConfig, OciKmsError, SystemEnvironment,
    };
}

//! Hyperswitch adapter over the shared [`oci_kms`](::oci_kms) client.
//!
//! The client itself (authentication, request signing, timeouts and retries) lives in the
//! standalone `oci_kms` crate so other services can share it. This adapter routes its clock
//! and entropy reads through `common_utils`, and adds Hyperswitch's logging and metrics.

use std::{sync::Arc, time::Instant};

use common_utils::errors::CustomResult;
use error_stack::{report, ResultExt};
pub use oci_kms::OciKmsConfig;
use router_env::logger;

use crate::metrics;

/// Client for OCI Vault KMS crypto operations.
#[derive(Clone, Debug)]
pub struct OciKmsClient {
    inner: oci_kms::OciKmsClient,
}

impl OciKmsClient {
    /// Constructs a new client; credentials are resolved lazily per call, not eagerly.
    pub async fn new(config: &OciKmsConfig) -> CustomResult<Self, OciKmsError> {
        let inner = oci_kms::OciKmsClient::with_environment(config, Arc::new(CommonUtilsSeams))
            .map_err(|error| report!(error).change_context(OciKmsError::ClientCreationFailed))?;
        Ok(Self { inner })
    }

    /// Decrypts base64-encoded ciphertext via OCI Vault KMS.
    pub async fn decrypt(&self, data: impl AsRef<[u8]>) -> CustomResult<String, OciKmsError> {
        let start = Instant::now();
        let ciphertext = std::str::from_utf8(data.as_ref())
            .change_context(OciKmsError::Utf8DecodingFailed)
            .attach_printable("Ciphertext input is not valid UTF-8")?;

        let plaintext = self.inner.decrypt(ciphertext).await.map_err(|error| {
            logger::error!(oci_kms_error = %error, "Failed to OCI KMS decrypt data");
            metrics::OCI_KMS_DECRYPTION_FAILURES.add(1, &[]);
            report!(error).change_context(OciKmsError::DecryptionFailed)
        })?;
        let output =
            String::from_utf8(plaintext).change_context(OciKmsError::Utf8DecodingFailed)?;

        metrics::OCI_KMS_DECRYPT_TIME.record(start.elapsed().as_secs_f64(), &[]);

        Ok(output)
    }

    /// Encrypts data via OCI Vault KMS, returning base64-encoded ciphertext.
    pub async fn encrypt(&self, data: impl AsRef<[u8]>) -> CustomResult<String, OciKmsError> {
        let start = Instant::now();

        let ciphertext = self.inner.encrypt(data.as_ref()).await.map_err(|error| {
            logger::error!(oci_kms_error = %error, "Failed to OCI KMS encrypt data");
            metrics::OCI_KMS_ENCRYPTION_FAILURES.add(1, &[]);
            report!(error).change_context(OciKmsError::EncryptionFailed)
        })?;

        metrics::OCI_KMS_ENCRYPT_TIME.record(start.elapsed().as_secs_f64(), &[]);

        Ok(ciphertext)
    }
}

/// Routes the client's clock and entropy reads through `common_utils`, so `deja` can replay
/// them.
#[derive(Debug)]
struct CommonUtilsSeams;

impl oci_kms::Environment for CommonUtilsSeams {
    fn now_unix_timestamp(&self) -> i64 {
        common_utils::date_time::now_unix_timestamp()
    }

    fn random_f64_unit(&self) -> f64 {
        common_utils::generate_random_f64_unit()
    }

    fn random_bytes(&self, len: usize) -> Vec<u8> {
        common_utils::generate_random_bytes(len)
    }
}

/// Errors that could occur during OCI Vault KMS operations.
#[derive(Debug, thiserror::Error)]
pub enum OciKmsError {
    /// An error occurred UTF-8 decoding input or output data.
    #[error("Failed UTF-8 decode of OCI KMS input/output data")]
    Utf8DecodingFailed,

    /// An error occurred when OCI KMS decrypting input data.
    #[error("Failed to OCI KMS decrypt input data")]
    DecryptionFailed,

    /// An error occurred when OCI KMS encrypting input data.
    #[error("Failed to OCI KMS encrypt input data")]
    EncryptionFailed,

    /// Failed while creating the OCI KMS client.
    #[error("Failed to create OCI KMS client")]
    ClientCreationFailed,
}

#[cfg(test)]
mod tests {
    use hyperswitch_interfaces::secrets_interface::SecretManagementInterface;
    use hyperswitch_masking::{PeekInterface, Secret};

    use super::*;

    fn env(name: &str) -> String {
        std::env::var(name).unwrap_or_else(|_| panic!("{name} must be set for live tests"))
    }

    /// The call Hyperswitch makes at startup for every secret in its config, through this
    /// adapter and its `common_utils` seams. Skipped by default; the `oci_kms` crate's `live`
    /// tests document the environment variables.
    #[tokio::test]
    #[ignore = "calls a real OCI Vault"]
    async fn get_secret_decrypts_a_config_secret() {
        let client = OciKmsClient::new(&OciKmsConfig {
            vault_crypto_endpoint: env("OCI_KMS_TEST_CRYPTO_ENDPOINT"),
            key_id: env("OCI_KMS_TEST_KEY_ID"),
        })
        .await
        .expect("client builds");

        let secret = SecretManagementInterface::get_secret(
            &client,
            Secret::new(env("OCI_KMS_TEST_CLI_CIPHERTEXT")),
        )
        .await
        .expect("get_secret succeeds");
        assert_eq!(secret.peek(), &env("OCI_KMS_TEST_CLI_PLAINTEXT"));
    }
}

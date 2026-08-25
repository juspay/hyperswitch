//! Interactions with the GCP Cloud KMS SDK

use std::{sync::Arc, time::Instant};

use base64::Engine;
use common_utils::errors::CustomResult;
use error_stack::ResultExt;
use google_cloud_kms::{
    client::{Client, ClientConfig},
    grpc::kms::v1::{DecryptRequest, EncryptRequest},
};
use router_env::logger;

use crate::{consts, metrics};

/// Configuration parameters required for constructing a [`GcpKmsClient`].
#[derive(Clone, Debug, Default, serde::Deserialize)]
#[serde(default)]
pub struct GcpKmsConfig {
    /// The GCP project ID that owns the KMS key ring.
    pub project_id: String,

    /// The location ID (e.g. `"global"`, `"us-east1"`) of the KMS key ring.
    pub location_id: String,

    /// The ID of the KMS key ring.
    pub key_ring_id: String,

    /// The ID of the KMS key used to encrypt or decrypt data.
    pub key_id: String,
}

impl GcpKmsConfig {
    /// Verifies that the [`GcpKmsClient`] configuration is usable. All four fields are
    /// required for both the secrets manager and encryption manager: GCP Cloud KMS
    /// ciphertext doesn't embed key identity, so both encrypt and decrypt need the full
    /// resource path.
    pub fn validate(&self) -> Result<(), &'static str> {
        use common_utils::{ext_traits::ConfigExt, fp_utils::when};

        when(self.project_id.is_default_or_empty(), || {
            Err("GCP KMS project ID must not be empty")
        })?;

        when(self.location_id.is_default_or_empty(), || {
            Err("GCP KMS location ID must not be empty")
        })?;

        when(self.key_ring_id.is_default_or_empty(), || {
            Err("GCP KMS key ring ID must not be empty")
        })?;

        when(self.key_id.is_default_or_empty(), || {
            Err("GCP KMS key ID must not be empty")
        })
    }
}

/// A thin seam between [`GcpKmsClient`] and the real `google-cloud-kms` client, so that
/// unit tests can substitute an in-process fake instead of hitting the real GCP KMS API
/// over the network.
#[async_trait::async_trait]
pub(crate) trait GcpKmsOperations: Send + Sync {
    /// Encrypts `plaintext` against the KMS key identified by `key_name`.
    async fn encrypt(
        &self,
        key_name: &str,
        plaintext: Vec<u8>,
    ) -> CustomResult<Vec<u8>, GcpKmsError>;

    /// Decrypts `ciphertext` against the KMS key identified by `key_name`.
    async fn decrypt(
        &self,
        key_name: &str,
        ciphertext: Vec<u8>,
    ) -> CustomResult<Vec<u8>, GcpKmsError>;
}

/// Wraps the real SDK client so [`GcpKmsOperations`] can be implemented on it without
/// its `encrypt`/`decrypt` method names colliding with the SDK's own — the same
/// wrap-as-a-field shape `AwsKmsClient`/`HashiCorpVault` already use for their SDK
/// clients, rather than implementing a trait directly on a type we don't own.
struct RealGcpKms(Client);

#[async_trait::async_trait]
impl GcpKmsOperations for RealGcpKms {
    async fn encrypt(
        &self,
        key_name: &str,
        plaintext: Vec<u8>,
    ) -> CustomResult<Vec<u8>, GcpKmsError> {
        let request = EncryptRequest {
            name: key_name.to_owned(),
            plaintext,
            additional_authenticated_data: Vec::new(),
            plaintext_crc32c: None,
            additional_authenticated_data_crc32c: None,
        };
        let response = self
            .0
            .encrypt(request, None)
            .await
            .change_context(GcpKmsError::EncryptionFailed)?;
        Ok(response.ciphertext)
    }

    async fn decrypt(
        &self,
        key_name: &str,
        ciphertext: Vec<u8>,
    ) -> CustomResult<Vec<u8>, GcpKmsError> {
        let request = DecryptRequest {
            name: key_name.to_owned(),
            ciphertext,
            additional_authenticated_data: Vec::new(),
            ciphertext_crc32c: None,
            additional_authenticated_data_crc32c: None,
        };
        let response = self
            .0
            .decrypt(request, None)
            .await
            .change_context(GcpKmsError::DecryptionFailed)?;
        Ok(response.plaintext)
    }
}

/// Client for GCP Cloud KMS operations.
#[derive(Clone)]
pub struct GcpKmsClient {
    inner: Arc<dyn GcpKmsOperations>,
    key_name: String,
}

impl std::fmt::Debug for GcpKmsClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GcpKmsClient")
            .field("key_name", &self.key_name)
            .finish()
    }
}

impl GcpKmsClient {
    /// Constructs a new GCP Cloud KMS client. Authenticates via Application Default
    /// Credentials (`GOOGLE_APPLICATION_CREDENTIALS` env var, or the GCE/GKE metadata
    /// server).
    pub async fn new(config: &GcpKmsConfig) -> CustomResult<Self, GcpKmsError> {
        let client_config = ClientConfig::default()
            .with_auth()
            .await
            .change_context(GcpKmsError::ClientCreationFailed)?;
        let client = Client::new(client_config)
            .await
            .change_context(GcpKmsError::ClientCreationFailed)?;
        Ok(Self {
            inner: Arc::new(RealGcpKms(client)),
            key_name: format!(
                "projects/{}/locations/{}/keyRings/{}/cryptoKeys/{}",
                config.project_id, config.location_id, config.key_ring_id, config.key_id
            ),
        })
    }

    #[cfg(test)]
    pub(crate) fn from_parts(inner: Arc<dyn GcpKmsOperations>, key_name: String) -> Self {
        Self { inner, key_name }
    }

    /// Decrypts base64-encoded ciphertext via GCP Cloud KMS.
    pub async fn decrypt(&self, data: impl AsRef<[u8]>) -> CustomResult<String, GcpKmsError> {
        let start = Instant::now();
        let ciphertext = consts::BASE64_ENGINE
            .decode(data)
            .change_context(GcpKmsError::Base64DecodingFailed)?;

        let plaintext = self
            .inner
            .decrypt(&self.key_name, ciphertext)
            .await
            .inspect_err(|error| {
                // Logging using `Debug` representation of the error as the `Display`
                // representation does not hold sufficient information.
                logger::error!(gcp_kms_error=?error, "Failed to GCP KMS decrypt data");
                metrics::GCP_KMS_DECRYPTION_FAILURES.add(1, &[]);
            })?;

        let output =
            String::from_utf8(plaintext).change_context(GcpKmsError::Utf8DecodingFailed)?;

        let time_taken = start.elapsed();
        metrics::GCP_KMS_DECRYPT_TIME.record(time_taken.as_secs_f64(), &[]);

        Ok(output)
    }

    /// Encrypts data via GCP Cloud KMS, returning base64-encoded ciphertext.
    pub async fn encrypt(&self, data: impl AsRef<[u8]>) -> CustomResult<String, GcpKmsError> {
        let start = Instant::now();
        let ciphertext = self
            .inner
            .encrypt(&self.key_name, data.as_ref().to_vec())
            .await
            .inspect_err(|error| {
                // Logging using `Debug` representation of the error as the `Display`
                // representation does not hold sufficient information.
                logger::error!(gcp_kms_error=?error, "Failed to GCP KMS encrypt data");
                metrics::GCP_KMS_ENCRYPTION_FAILURES.add(1, &[]);
            })?;

        let output = consts::BASE64_ENGINE.encode(ciphertext);

        let time_taken = start.elapsed();
        metrics::GCP_KMS_ENCRYPT_TIME.record(time_taken.as_secs_f64(), &[]);

        Ok(output)
    }
}

/// Errors that could occur during GCP Cloud KMS operations. `EncryptResponse`/
/// `DecryptResponse` fields are plain `Vec<u8>` (never absent on success), and
/// `key_id` is validated non-empty before a client is ever constructed — so there's no
/// "missing output" or "missing key id" variant to model here.
#[derive(Debug, thiserror::Error)]
pub enum GcpKmsError {
    /// An error occurred when base64 decoding input data.
    #[error("Failed to base64 decode input data")]
    Base64DecodingFailed,

    /// An error occurred when GCP KMS decrypting input data.
    #[error("Failed to GCP KMS decrypt input data")]
    DecryptionFailed,

    /// An error occurred when GCP KMS encrypting input data.
    #[error("Failed to GCP KMS encrypt input data")]
    EncryptionFailed,

    /// An error occurred UTF-8 decoding GCP KMS decrypted output.
    #[error("Failed UTF-8 decode of GCP KMS decrypted output")]
    Utf8DecodingFailed,

    /// Failed while creating the GCP KMS client.
    #[error("Failed to create GCP KMS client")]
    ClientCreationFailed,
}

#[cfg(test)]
mod tests {
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    use error_stack::report;

    use super::*;

    /// A deterministic, in-process fake for [`GcpKmsOperations`], so that unit tests
    /// exercise `GcpKmsClient`'s base64/UTF-8/error-mapping logic without any network
    /// access or real GCP credentials.
    #[derive(Clone, Default)]
    struct FakeGcpKmsOperations {
        /// Bytes to return from `decrypt`, overriding the default passthrough
        /// (returning the ciphertext bytes unchanged) behaviour.
        decrypt_output: Option<Vec<u8>>,
        /// If `true`, `encrypt` fails with [`GcpKmsError::EncryptionFailed`].
        fail_encrypt: bool,
        /// If `true`, `decrypt` fails with [`GcpKmsError::DecryptionFailed`].
        fail_decrypt: bool,
        encrypt_calls: Arc<AtomicUsize>,
        decrypt_calls: Arc<AtomicUsize>,
    }

    #[async_trait::async_trait]
    impl GcpKmsOperations for FakeGcpKmsOperations {
        async fn encrypt(
            &self,
            _key_name: &str,
            plaintext: Vec<u8>,
        ) -> CustomResult<Vec<u8>, GcpKmsError> {
            self.encrypt_calls.fetch_add(1, Ordering::SeqCst);
            if self.fail_encrypt {
                return Err(report!(GcpKmsError::EncryptionFailed));
            }
            // Fake "encryption": return the plaintext bytes unchanged. Good enough to
            // exercise the base64 boundary in `GcpKmsClient::encrypt`/`decrypt`.
            Ok(plaintext)
        }

        async fn decrypt(
            &self,
            _key_name: &str,
            ciphertext: Vec<u8>,
        ) -> CustomResult<Vec<u8>, GcpKmsError> {
            self.decrypt_calls.fetch_add(1, Ordering::SeqCst);
            if self.fail_decrypt {
                return Err(report!(GcpKmsError::DecryptionFailed));
            }
            Ok(self.decrypt_output.clone().unwrap_or(ciphertext))
        }
    }

    fn client_with(fake: FakeGcpKmsOperations) -> GcpKmsClient {
        GcpKmsClient::from_parts(Arc::new(fake), "test-key-name".to_string())
    }

    #[tokio::test]
    async fn encrypt_decrypt_round_trip_through_base64_boundary() {
        let client = client_with(FakeGcpKmsOperations::default());

        let plaintext = "hyperswitch-secret-value";
        let encrypted = client
            .encrypt(plaintext.as_bytes())
            .await
            .expect("encryption should succeed");

        // The output of `encrypt` must be valid base64, distinct in general from the
        // raw plaintext bytes representation.
        assert!(base64::engine::general_purpose::STANDARD
            .decode(&encrypted)
            .is_ok());

        let decrypted = client
            .decrypt(encrypted.as_bytes())
            .await
            .expect("decryption should succeed");

        assert_eq!(decrypted, plaintext);
    }

    #[tokio::test]
    async fn decrypt_with_malformed_base64_input_fails_with_base64_decoding_failed() {
        let client = client_with(FakeGcpKmsOperations::default());

        let error = client
            .decrypt(b"not-valid-base64!!!")
            .await
            .expect_err("malformed base64 input should fail");

        assert!(matches!(
            error.current_context(),
            GcpKmsError::Base64DecodingFailed
        ));
    }

    #[tokio::test]
    async fn decrypt_with_non_utf8_output_fails_with_utf8_decoding_failed() {
        // Bytes that are not valid UTF-8.
        let non_utf8_bytes = vec![0xFF, 0xFE, 0xFD];
        let fake = FakeGcpKmsOperations {
            decrypt_output: Some(non_utf8_bytes),
            ..FakeGcpKmsOperations::default()
        };
        let client = client_with(fake);

        let ciphertext = consts::BASE64_ENGINE.encode(b"irrelevant-input");
        let error = client
            .decrypt(ciphertext.as_bytes())
            .await
            .expect_err("non-UTF-8 decrypted output should fail");

        assert!(matches!(
            error.current_context(),
            GcpKmsError::Utf8DecodingFailed
        ));
    }

    #[tokio::test]
    async fn decrypt_propagates_backend_failure_as_decryption_failed() {
        let fake = FakeGcpKmsOperations {
            fail_decrypt: true,
            ..FakeGcpKmsOperations::default()
        };
        let client = client_with(fake);

        let ciphertext = consts::BASE64_ENGINE.encode(b"irrelevant-input");
        let error = client
            .decrypt(ciphertext.as_bytes())
            .await
            .expect_err("backend decrypt failure should propagate");

        assert!(matches!(
            error.current_context(),
            GcpKmsError::DecryptionFailed
        ));
    }

    #[tokio::test]
    async fn encrypt_propagates_backend_failure_as_encryption_failed() {
        let fake = FakeGcpKmsOperations {
            fail_encrypt: true,
            ..FakeGcpKmsOperations::default()
        };
        let client = client_with(fake);

        let error = client
            .encrypt(b"irrelevant-input")
            .await
            .expect_err("backend encrypt failure should propagate");

        assert!(matches!(
            error.current_context(),
            GcpKmsError::EncryptionFailed
        ));
    }

    #[test]
    fn validate_fails_when_project_id_is_empty() {
        let config = GcpKmsConfig {
            project_id: String::new(),
            location_id: "global".to_string(),
            key_ring_id: "key-ring".to_string(),
            key_id: "key".to_string(),
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn validate_fails_when_location_id_is_empty() {
        let config = GcpKmsConfig {
            project_id: "project".to_string(),
            location_id: String::new(),
            key_ring_id: "key-ring".to_string(),
            key_id: "key".to_string(),
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn validate_fails_when_key_ring_id_is_empty() {
        let config = GcpKmsConfig {
            project_id: "project".to_string(),
            location_id: "global".to_string(),
            key_ring_id: String::new(),
            key_id: "key".to_string(),
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn validate_fails_when_key_id_is_empty() {
        let config = GcpKmsConfig {
            project_id: "project".to_string(),
            location_id: "global".to_string(),
            key_ring_id: "key-ring".to_string(),
            key_id: String::new(),
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn validate_succeeds_when_all_fields_are_populated() {
        let config = GcpKmsConfig {
            project_id: "project".to_string(),
            location_id: "global".to_string(),
            key_ring_id: "key-ring".to_string(),
            key_id: "key".to_string(),
        };
        assert!(config.validate().is_ok());
    }

    /// Manual, real-credential sanity test mirroring `aws_kms::core::tests`.
    /// Run locally with `GOOGLE_APPLICATION_CREDENTIALS` set to a real service-account
    /// JSON key and a real `project_id`/`location_id`/`key_ring_id`/`key_id`, via:
    ///
    /// `cargo test -p external_services --features gcp_kms check_gcp_kms -- --nocapture`
    ///
    /// This is excluded from default CI and from the automated regression coverage
    /// above (which is the primary, CI-capable protection this backend has) — it is a
    /// secondary safety net only, and will fail without real GCP credentials/config.
    #[tokio::test]
    #[ignore = "requires real GCP credentials and a real KMS key; run manually"]
    async fn check_gcp_kms_encrypt() {
        let config = GcpKmsConfig {
            project_id: "YOUR GCP PROJECT ID".to_string(),
            location_id: "YOUR GCP KMS LOCATION ID".to_string(),
            key_ring_id: "YOUR GCP KMS KEY RING ID".to_string(),
            key_id: "YOUR GCP KMS KEY ID".to_string(),
        };

        let data = "hello".to_string();
        let client = GcpKmsClient::new(&config)
            .await
            .expect("gcp kms client creation failed");
        let gcp_kms_encrypted_fingerprint = client
            .encrypt(data.as_bytes())
            .await
            .expect("gcp kms encryption failed");

        println!("{gcp_kms_encrypted_fingerprint}");
    }

    #[tokio::test]
    #[ignore = "requires real GCP credentials and a real KMS key; run manually"]
    async fn check_gcp_kms_decrypt() {
        let config = GcpKmsConfig {
            project_id: "YOUR GCP PROJECT ID".to_string(),
            location_id: "YOUR GCP KMS LOCATION ID".to_string(),
            key_ring_id: "YOUR GCP KMS KEY RING ID".to_string(),
            key_id: "YOUR GCP KMS KEY ID".to_string(),
        };

        // Should decrypt to hello
        let data = "GCP KMS ENCRYPTED CIPHER".to_string();
        let client = GcpKmsClient::new(&config)
            .await
            .expect("gcp kms client creation failed");
        let gcp_kms_decrypted_fingerprint = client
            .decrypt(data.as_bytes())
            .await
            .expect("gcp kms decryption failed");

        println!("{gcp_kms_decrypted_fingerprint}");
    }
}

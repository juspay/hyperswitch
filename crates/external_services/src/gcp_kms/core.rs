//! Interactions with the GCP Cloud KMS SDK

use std::time::Instant;

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
    /// Verifies that the [`GcpKmsConfig`] is valid.
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

/// Client for GCP Cloud KMS operations.
#[derive(Clone, Debug)]
pub struct GcpKmsClient {
    inner_client: Client,
    key_name: String,
}

impl GcpKmsClient {
    /// Constructs a new GCP KMS client with ambient credentials, targeting the KMS key
    /// identified by the provided [`GcpKmsConfig`].
    pub async fn new(config: &GcpKmsConfig) -> CustomResult<Self, GcpKmsError> {
        let client_config = ClientConfig::default()
            .with_auth()
            .await
            .change_context(GcpKmsError::ClientCreationFailed)?;
        let inner_client = Client::new(client_config)
            .await
            .change_context(GcpKmsError::ClientCreationFailed)?;
        Ok(Self {
            inner_client,
            key_name: format!(
                "projects/{}/locations/{}/keyRings/{}/cryptoKeys/{}",
                config.project_id, config.location_id, config.key_ring_id, config.key_id
            ),
        })
    }

    /// Decrypts base64-encoded ciphertext via GCP Cloud KMS.
    pub async fn decrypt(&self, data: impl AsRef<[u8]>) -> CustomResult<String, GcpKmsError> {
        let start = Instant::now();
        let ciphertext = consts::BASE64_ENGINE
            .decode(data)
            .change_context(GcpKmsError::Base64DecodingFailed)?;

        let request = DecryptRequest {
            name: self.key_name.clone(),
            ciphertext,
            additional_authenticated_data: Vec::new(),
            ciphertext_crc32c: None,
            additional_authenticated_data_crc32c: None,
        };

        let response = self
            .inner_client
            .decrypt(request, None)
            .await
            .inspect_err(|error| {
                logger::error!(gcp_kms_error=?error, "Failed to GCP KMS decrypt data");
                metrics::GCP_KMS_DECRYPTION_FAILURES.add(1, &[]);
            })
            .change_context(GcpKmsError::DecryptionFailed)?;

        let output = String::from_utf8(response.plaintext)
            .change_context(GcpKmsError::Utf8DecodingFailed)?;

        let time_taken = start.elapsed();
        metrics::GCP_KMS_DECRYPT_TIME.record(time_taken.as_secs_f64(), &[]);

        Ok(output)
    }

    /// Encrypts data via GCP Cloud KMS, returning base64-encoded ciphertext.
    pub async fn encrypt(&self, data: impl AsRef<[u8]>) -> CustomResult<String, GcpKmsError> {
        let start = Instant::now();

        let request = EncryptRequest {
            name: self.key_name.clone(),
            plaintext: data.as_ref().to_vec(),
            additional_authenticated_data: Vec::new(),
            plaintext_crc32c: None,
            additional_authenticated_data_crc32c: None,
        };

        let response = self
            .inner_client
            .encrypt(request, None)
            .await
            .inspect_err(|error| {
                logger::error!(gcp_kms_error=?error, "Failed to GCP KMS encrypt data");
                metrics::GCP_KMS_ENCRYPTION_FAILURES.add(1, &[]);
            })
            .change_context(GcpKmsError::EncryptionFailed)?;

        let output = consts::BASE64_ENGINE.encode(response.ciphertext);

        let time_taken = start.elapsed();
        metrics::GCP_KMS_ENCRYPT_TIME.record(time_taken.as_secs_f64(), &[]);

        Ok(output)
    }
}

/// Errors that could occur during GCP KMS operations.
#[derive(Debug, thiserror::Error)]
pub enum GcpKmsError {
    /// An error occurred when base64 decoding the input data.
    #[error("Failed to base64 decode input data")]
    Base64DecodingFailed,

    /// An error occurred when GCP KMS decrypting the input data.
    #[error("Failed to GCP KMS decrypt input data")]
    DecryptionFailed,

    /// An error occurred when GCP KMS encrypting the input data.
    #[error("Failed to GCP KMS encrypt input data")]
    EncryptionFailed,

    /// An error occurred UTF-8 decoding the GCP KMS decrypted output.
    #[error("Failed UTF-8 decode of GCP KMS decrypted output")]
    Utf8DecodingFailed,

    /// An error occurred when creating the GCP KMS client.
    #[error("Failed to create GCP KMS client")]
    ClientCreationFailed,
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[tokio::test]
    async fn check_gcp_kms_encrypt() {
        let config = GcpKmsConfig {
            project_id: "YOUR GCP PROJECT ID".to_string(),
            location_id: "YOUR GCP KMS LOCATION ID".to_string(),
            key_ring_id: "YOUR GCP KMS KEY RING ID".to_string(),
            key_id: "YOUR GCP KMS KEY ID".to_string(),
        };

        let data = "hello".to_string();
        let gcp_kms_encrypted_fingerprint = GcpKmsClient::new(&config)
            .await
            .expect("gcp kms client creation failed")
            .encrypt(data.as_bytes())
            .await
            .expect("gcp kms encryption failed");

        println!("{gcp_kms_encrypted_fingerprint}");
    }

    #[tokio::test]
    async fn check_gcp_kms_decrypt() {
        let config = GcpKmsConfig {
            project_id: "YOUR GCP PROJECT ID".to_string(),
            location_id: "YOUR GCP KMS LOCATION ID".to_string(),
            key_ring_id: "YOUR GCP KMS KEY RING ID".to_string(),
            key_id: "YOUR GCP KMS KEY ID".to_string(),
        };

        // Should decrypt to hello
        let data = "GCP KMS ENCRYPTED CIPHER".to_string();
        let gcp_kms_decrypted_fingerprint = GcpKmsClient::new(&config)
            .await
            .expect("gcp kms client creation failed")
            .decrypt(data.as_bytes())
            .await
            .expect("gcp kms decryption failed");

        println!("{gcp_kms_decrypted_fingerprint}");
    }
}

//! Interactions with Google Cloud KMS

use std::time::Instant;

use base64::Engine;
use common_utils::errors::CustomResult;
use error_stack::ResultExt;
use google_cloud_kms::client::{Client, ClientConfig};
use google_cloud_kms::grpc::kms::v1::{DecryptRequest, EncryptRequest};
use router_env::logger;

use crate::{consts, metrics};

/// Configuration parameters required for constructing a [`GcpKmsClient`]
#[derive(Clone, Debug, Default, serde::Deserialize)]
#[serde(default)]
pub struct GcpKmsConfig {
    /// GCP Project ID
    pub project_id: String,
    /// Location of the key ring
    pub location: String,
    /// Name of the key ring
    pub key_ring: String,
    /// Name of the crypto key
    pub key_name: String,
}

/// Client for Google Cloud KMS operations
#[derive(Debug, Clone)]
pub struct GcpKmsClient {
    inner_client: Client,
    key_resource_name: String,
}

impl GcpKmsClient {
    /// Constructs a new Google Cloud KMS client
    pub async fn new(config: &GcpKmsConfig) -> CustomResult<Self, GcpKmsError> {
        let key_resource_name = format!(
            "projects/{}/locations/{}/keyRings/{}/cryptoKeys/{}",
            config.project_id, config.location, config.key_ring, config.key_name
        );

        // Create KMS client
        let client_config = ClientConfig::default().with_auth().await
            .change_context(GcpKmsError::ClientInitializationFailed)?;
        let inner_client = Client::new(client_config).await
            .change_context(GcpKmsError::ClientInitializationFailed)?;

        Ok(Self {
            inner_client,
            key_resource_name,
        })
    }

    /// Encrypts plaintext using Google Cloud KMS
    pub async fn encrypt(&self, data: &[u8]) -> CustomResult<String, GcpKmsError> {
        let start = Instant::now();

        let request = EncryptRequest {
            name: self.key_resource_name.clone(),
            plaintext: data.to_vec(),
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

        let encoded = consts::BASE64_ENGINE.encode(&response.ciphertext);

        let time_taken = start.elapsed();
        metrics::GCP_KMS_ENCRYPT_TIME.record(time_taken.as_secs_f64(), &[]);

        Ok(encoded)
    }

    /// Decrypts ciphertext using Google Cloud KMS
    pub async fn decrypt(&self, data: &[u8]) -> CustomResult<String, GcpKmsError> {
        let start = Instant::now();
        let ciphertext = consts::BASE64_ENGINE
            .decode(data)
            .change_context(GcpKmsError::Base64DecodingFailed)?;

        let request = DecryptRequest {
            name: self.key_resource_name.clone(),
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

        let output =
            String::from_utf8(response.plaintext).change_context(GcpKmsError::Utf8DecodingFailed)?;

        let time_taken = start.elapsed();
        metrics::GCP_KMS_DECRYPT_TIME.record(time_taken.as_secs_f64(), &[]);

        Ok(output)
    }
}

/// Errors that could occur during GCP KMS operations
#[derive(Debug, thiserror::Error)]
pub enum GcpKmsError {
    /// Failed to initialize GCP KMS client
    #[error("Failed to initialize GCP KMS client")]
    ClientInitializationFailed,

    /// Failed to initialize GCP KMS client
    #[error("Failed to initialize GCP KMS client")]
    InitializationFailed,

    /// Failed to base64 encode input data
    #[error("Failed to base64 encode input data")]
    Base64EncodingFailed,

    /// Failed to base64 decode input data
    #[error("Failed to base64 decode input data")]
    Base64DecodingFailed,

    /// Failed to GCP KMS decrypt input data
    #[error("Failed to GCP KMS decrypt input data")]
    DecryptionFailed,

    /// Failed to GCP KMS encrypt input data
    #[error("Failed to GCP KMS encrypt input data")]
    EncryptionFailed,

    /// Missing plaintext in decryption output
    #[error("Missing plaintext in decryption output")]
    MissingPlaintext,

    /// Failed to UTF-8 decode decryption output
    #[error("Failed to UTF-8 decode decryption output")]
    Utf8DecodingFailed,
}

impl GcpKmsConfig {
    /// Verifies that the configuration is usable
    pub fn validate(&self) -> Result<(), &'static str> {
        use common_utils::{ext_traits::ConfigExt, fp_utils::when};

        when(self.project_id.is_default_or_empty(), || {
            Err("GCP project ID must not be empty")
        })?;

        when(self.location.is_default_or_empty(), || {
            Err("GCP location must not be empty")
        })?;

        when(self.key_ring.is_default_or_empty(), || {
            Err("GCP key ring must not be empty")
        })?;

        when(self.key_name.is_default_or_empty(), || {
            Err("GCP key name must not be empty")
        })
    }
}

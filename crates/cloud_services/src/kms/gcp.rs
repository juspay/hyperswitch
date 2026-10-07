//! Interactions with the GCP Cloud KMS SDK

use base64::Engine;
use google_cloud_gax::{conn::Error as ConnectionError, grpc::Status};
use google_cloud_kms::{
    client::{google_cloud_auth::error::Error as AuthError, Client, ClientConfig},
    grpc::kms::v1::{DecryptRequest, EncryptRequest},
};

/// Base64 engine used to encode ciphertext returned to, and decode ciphertext received from, callers.
const BASE64_ENGINE: base64::engine::GeneralPurpose = base64::engine::general_purpose::STANDARD;

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
        if self.project_id.trim().is_empty() {
            return Err("GCP KMS project ID must not be empty");
        }

        if self.location_id.trim().is_empty() {
            return Err("GCP KMS location ID must not be empty");
        }

        if self.key_ring_id.trim().is_empty() {
            return Err("GCP KMS key ring ID must not be empty");
        }

        if self.key_id.trim().is_empty() {
            return Err("GCP KMS key ID must not be empty");
        }

        Ok(())
    }

    /// Resource name of the configured key.
    fn key_name(&self) -> String {
        format!(
            "projects/{}/locations/{}/keyRings/{}/cryptoKeys/{}",
            self.project_id, self.location_id, self.key_ring_id, self.key_id
        )
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
    pub async fn new(config: &GcpKmsConfig) -> Result<Self, GcpKmsError> {
        let client_config = ClientConfig::default()
            .with_auth()
            .await
            .map_err(GcpKmsError::CredentialsUnavailable)?;
        let inner_client = Client::new(client_config)
            .await
            .map_err(GcpKmsError::ClientCreationFailed)?;
        Ok(Self {
            inner_client,
            key_name: config.key_name(),
        })
    }

    /// Decrypts base64-encoded ciphertext via GCP Cloud KMS.
    pub async fn decrypt(&self, data: impl AsRef<[u8]>) -> Result<String, GcpKmsError> {
        let ciphertext = BASE64_ENGINE
            .decode(data)
            .map_err(GcpKmsError::Base64DecodingFailed)?;

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
            .map_err(GcpKmsError::DecryptionFailed)?;

        String::from_utf8(response.plaintext).map_err(GcpKmsError::Utf8DecodingFailed)
    }

    /// Encrypts data via GCP Cloud KMS, returning base64-encoded ciphertext.
    pub async fn encrypt(&self, data: impl AsRef<[u8]>) -> Result<String, GcpKmsError> {
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
            .map_err(GcpKmsError::EncryptionFailed)?;

        Ok(BASE64_ENGINE.encode(response.ciphertext))
    }
}

/// Errors that could occur during GCP KMS operations.
#[derive(Debug, thiserror::Error)]
pub enum GcpKmsError {
    /// An error occurred when base64 decoding the input data.
    #[error("Failed to base64 decode input data")]
    Base64DecodingFailed(#[source] base64::DecodeError),

    /// An error occurred when GCP KMS decrypting the input data.
    #[error("Failed to GCP KMS decrypt input data")]
    DecryptionFailed(#[source] Status),

    /// An error occurred when GCP KMS encrypting the input data.
    #[error("Failed to GCP KMS encrypt input data")]
    EncryptionFailed(#[source] Status),

    /// An error occurred UTF-8 decoding the GCP KMS decrypted output.
    #[error("Failed UTF-8 decode of GCP KMS decrypted output")]
    Utf8DecodingFailed(#[source] std::string::FromUtf8Error),

    /// Ambient GCP credentials could not be loaded while creating the client.
    #[error("Failed to load GCP credentials for the KMS client")]
    CredentialsUnavailable(#[source] AuthError),

    /// An error occurred when creating the GCP KMS client.
    #[error("Failed to create GCP KMS client")]
    ClientCreationFailed(#[source] ConnectionError),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> GcpKmsConfig {
        GcpKmsConfig {
            project_id: "project".to_owned(),
            location_id: "global".to_owned(),
            key_ring_id: "key-ring".to_owned(),
            key_id: "key".to_owned(),
        }
    }

    #[test]
    fn validate_reports_the_first_missing_field() {
        assert!(config().validate().is_ok());

        type ClearField = fn(&mut GcpKmsConfig);

        let cases: [(ClearField, &str); 4] = [
            (
                |c| c.project_id = " ".to_owned(),
                "GCP KMS project ID must not be empty",
            ),
            (
                |c| c.location_id.clear(),
                "GCP KMS location ID must not be empty",
            ),
            (
                |c| c.key_ring_id.clear(),
                "GCP KMS key ring ID must not be empty",
            ),
            (|c| c.key_id.clear(), "GCP KMS key ID must not be empty"),
        ];
        for (clear_field, message) in cases {
            let mut config = config();
            clear_field(&mut config);
            assert_eq!(config.validate(), Err(message));
        }
    }

    #[test]
    fn key_name_is_the_full_resource_name() {
        assert_eq!(
            config().key_name(),
            "projects/project/locations/global/keyRings/key-ring/cryptoKeys/key"
        );
    }
}

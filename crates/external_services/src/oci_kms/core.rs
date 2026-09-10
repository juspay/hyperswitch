//! Interactions with the OCI Vault KMS crypto endpoint

use std::{sync::Arc, time::Instant};

use base64::Engine;
use common_utils::errors::CustomResult;
use error_stack::{report, ResultExt};
use router_env::logger;
use serde::{Deserialize, Serialize};

use super::{credentials::CredentialCache, signing};
use crate::{consts, metrics};

/// Configuration parameters required for constructing an [`OciKmsClient`].
#[derive(Clone, Debug, Default, serde::Deserialize)]
#[serde(default)]
pub struct OciKmsConfig {
    /// The vault's crypto endpoint (e.g. `https://<vault>-crypto.kms.<region>.oraclecloud.com`),
    /// obtained once when the vault is created; doesn't change afterward.
    pub vault_crypto_endpoint: String,

    /// The full OCID of the KMS key used to encrypt/decrypt data.
    pub key_id: String,
}

impl OciKmsConfig {
    /// Verifies that the [`OciKmsClient`] configuration is usable.
    pub fn validate(&self) -> Result<(), &'static str> {
        use common_utils::{ext_traits::ConfigExt, fp_utils::when};

        when(self.vault_crypto_endpoint.is_default_or_empty(), || {
            Err("OCI KMS vault crypto endpoint must not be empty")
        })?;

        when(self.key_id.is_default_or_empty(), || {
            Err("OCI KMS key ID must not be empty")
        })
    }
}

#[derive(Serialize)]
struct EncryptDataDetails<'a> {
    #[serde(rename = "keyId")]
    key_id: &'a str,
    plaintext: String,
    #[serde(rename = "encryptionAlgorithm")]
    encryption_algorithm: &'static str,
}

#[derive(Serialize)]
struct DecryptDataDetails<'a> {
    #[serde(rename = "keyId")]
    key_id: &'a str,
    ciphertext: &'a str,
    #[serde(rename = "encryptionAlgorithm")]
    encryption_algorithm: &'static str,
}

#[derive(Deserialize)]
struct EncryptedData {
    ciphertext: String,
}

#[derive(Deserialize)]
struct DecryptedData {
    plaintext: String,
}

const ENCRYPTION_ALGORITHM: &str = "AES_256_GCM";

/// Client for OCI Vault KMS crypto operations; see `workload_identity` for auth.
#[derive(Clone)]
pub struct OciKmsClient {
    http_client: reqwest::Client,
    vault_crypto_endpoint: String,
    host: String,
    key_id: String,
    credentials: Arc<CredentialCache>,
}

impl std::fmt::Debug for OciKmsClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OciKmsClient")
            .field("vault_crypto_endpoint", &self.vault_crypto_endpoint)
            .field("key_id", &self.key_id)
            .finish()
    }
}

impl OciKmsClient {
    /// Constructs a new client; credentials are resolved lazily per call, not eagerly.
    pub async fn new(config: &OciKmsConfig) -> CustomResult<Self, OciKmsError> {
        let host = url::Url::parse(&config.vault_crypto_endpoint)
            .change_context(OciKmsError::ClientCreationFailed)
            .attach_printable("Invalid OCI KMS vault crypto endpoint URL")?
            .host_str()
            .ok_or_else(|| report!(OciKmsError::ClientCreationFailed))
            .attach_printable("OCI KMS vault crypto endpoint URL has no host")?
            .to_owned();

        Ok(Self {
            http_client: reqwest::Client::new(),
            vault_crypto_endpoint: config
                .vault_crypto_endpoint
                .trim_end_matches('/')
                .to_owned(),
            host,
            key_id: config.key_id.clone(),
            credentials: Arc::new(CredentialCache::default()),
        })
    }

    /// Decrypts base64-encoded ciphertext via OCI Vault KMS.
    pub async fn decrypt(&self, data: impl AsRef<[u8]>) -> CustomResult<String, OciKmsError> {
        let start = Instant::now();
        let ciphertext = std::str::from_utf8(data.as_ref())
            .change_context(OciKmsError::Utf8DecodingFailed)
            .attach_printable("Ciphertext input is not valid UTF-8")?;

        let request = DecryptDataDetails {
            key_id: &self.key_id,
            ciphertext,
            encryption_algorithm: ENCRYPTION_ALGORITHM,
        };

        let response: DecryptedData = self
            .call("/20180608/decrypt", &request)
            .await
            .inspect_err(|error| {
                logger::error!(oci_kms_error=?error, "Failed to OCI KMS decrypt data");
                metrics::OCI_KMS_DECRYPTION_FAILURES.add(1, &[]);
            })
            .change_context(OciKmsError::DecryptionFailed)?;

        let plaintext = consts::BASE64_ENGINE
            .decode(response.plaintext)
            .change_context(OciKmsError::Base64DecodingFailed)?;
        let output =
            String::from_utf8(plaintext).change_context(OciKmsError::Utf8DecodingFailed)?;

        metrics::OCI_KMS_DECRYPT_TIME.record(start.elapsed().as_secs_f64(), &[]);

        Ok(output)
    }

    /// Encrypts data via OCI Vault KMS, returning base64-encoded ciphertext.
    pub async fn encrypt(&self, data: impl AsRef<[u8]>) -> CustomResult<String, OciKmsError> {
        let start = Instant::now();
        let plaintext = consts::BASE64_ENGINE.encode(data.as_ref());

        let request = EncryptDataDetails {
            key_id: &self.key_id,
            plaintext,
            encryption_algorithm: ENCRYPTION_ALGORITHM,
        };

        let response: EncryptedData = self
            .call("/20180608/encrypt", &request)
            .await
            .inspect_err(|error| {
                logger::error!(oci_kms_error=?error, "Failed to OCI KMS encrypt data");
                metrics::OCI_KMS_ENCRYPTION_FAILURES.add(1, &[]);
            })
            .change_context(OciKmsError::EncryptionFailed)?;

        metrics::OCI_KMS_ENCRYPT_TIME.record(start.elapsed().as_secs_f64(), &[]);

        Ok(response.ciphertext)
    }

    async fn call<Request, Response>(
        &self,
        path: &str,
        request: &Request,
    ) -> CustomResult<Response, OciKmsError>
    where
        Request: Serialize,
        Response: serde::de::DeserializeOwned,
    {
        let credentials = self.credentials.current().await?;
        let body = serde_json::to_vec(request)
            .change_context(OciKmsError::ClientCreationFailed)
            .attach_printable("Failed to serialize OCI KMS request body")?;

        let signed = signing::sign_post_request(
            &credentials.key_id,
            &credentials.private_key,
            &self.host,
            path,
            &body,
        )?;

        let response = self
            .http_client
            .post(format!("{}{path}", self.vault_crypto_endpoint))
            .header("date", signed.date)
            .header("authorization", signed.authorization)
            .header("content-type", "application/json")
            .header("x-content-sha256", signed.x_content_sha256)
            .body(body)
            .send()
            .await
            .change_context(OciKmsError::RequestFailed)
            .attach_printable("Failed to send OCI KMS request")?;

        let status = response.status();
        let response_body = response
            .text()
            .await
            .change_context(OciKmsError::RequestFailed)
            .attach_printable("Failed to read OCI KMS response body")?;

        if !status.is_success() {
            return Err(report!(OciKmsError::RequestFailed)).attach_printable(format!(
                "OCI KMS request failed with status {status}: {response_body}"
            ));
        }

        serde_json::from_str(&response_body)
            .change_context(OciKmsError::RequestFailed)
            .attach_printable("Failed to parse OCI KMS response body")
    }
}

/// Errors that could occur during OCI Vault KMS operations.
#[derive(Debug, thiserror::Error)]
pub enum OciKmsError {
    /// An error occurred when base64 decoding input data.
    #[error("Failed to base64 decode input data")]
    Base64DecodingFailed,

    /// An error occurred UTF-8 decoding input or output data.
    #[error("Failed UTF-8 decode of OCI KMS input/output data")]
    Utf8DecodingFailed,

    /// An error occurred when OCI KMS decrypting input data.
    #[error("Failed to OCI KMS decrypt input data")]
    DecryptionFailed,

    /// An error occurred when OCI KMS encrypting input data.
    #[error("Failed to OCI KMS encrypt input data")]
    EncryptionFailed,

    /// Constructing the OCI Signature v1 `Authorization` header failed.
    #[error("Failed to sign OCI KMS request")]
    SigningFailed,

    /// Workload Identity credentials couldn't be obtained from the OKE proxymux service.
    #[error("OCI Workload Identity credentials unavailable")]
    CredentialsUnavailable,

    /// The crypto-endpoint request failed, returned a non-success status, or its response body couldn't be parsed.
    #[error("OCI KMS request failed")]
    RequestFailed,

    /// Failed while creating the OCI KMS client.
    #[error("Failed to create OCI KMS client")]
    ClientCreationFailed,
}
